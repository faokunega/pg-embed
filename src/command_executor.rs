//! Generic async process runner used by the pg_ctl and initdb wrappers.
//!
//! The core abstraction is [`AsyncCommand`], a trait with two methods:
//! [`AsyncCommand::new`] spawns the OS process and [`AsyncCommand::execute`]
//! waits for it to finish (with an optional timeout).
//!
//! [`ProcessStatus`] is a companion trait that maps a process type (initdb,
//! start, stop) to the status values and errors it should produce.
//!
//! The only concrete implementation is [`AsyncCommandExecutor`].

use std::error::Error;
use std::ffi::OsStr;
use std::marker;
use std::process::Stdio;

use log;
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::process::Child;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::time::Duration;

/// Indicates whether a log line came from stdout or stderr.
#[derive(Debug)]
pub enum LogType {
    /// Standard output line.
    Info,
    /// Standard error line.
    Error,
}

/// Maps a process type to the status values and errors it should produce.
///
/// Implement this trait on an enum whose variants represent the distinct
/// processes you want to run (e.g. `InitDb`, `StartDb`, `StopDb`).  The
/// executor calls these methods to produce typed status/error values without
/// knowing the concrete error type.
pub trait ProcessStatus<T, E>
where
    E: Error + Send,
    Self: Send,
{
    /// Returns the status value that signals the process has *entered* execution.
    fn status_entry(&self) -> T;

    /// Returns the status value that signals the process *exited successfully*.
    fn status_exit(&self) -> T;

    /// Returns the error value for a generic process failure (non-zero exit,
    /// spawn error, etc.).
    fn error_type(&self) -> E;

    /// Returns the error value to use when the process exceeds its timeout.
    ///
    /// Defaults to [`Self::error_type`].  Override to return a distinct
    /// timeout-specific error variant.
    fn timeout_error(&self) -> E {
        self.error_type()
    }

    /// Wraps a foreign error `F` (e.g. an OS I/O error) into `E`, optionally
    /// attaching a context `message`.
    fn wrap_error<F: Error + Sync + Send + 'static>(&self, error: F, message: Option<String>) -> E;
}

/// A single log line captured from a child process stream.
#[derive(Debug)]
pub struct LogOutputData {
    line: String,
    log_type: LogType,
}

/// Trait for types that can spawn and execute an OS process asynchronously.
///
/// The type parameter `S` is the success-status type (e.g. [`crate::pg_enums::PgServerStatus`]),
/// `E` is the error type, and `P` is the [`ProcessStatus`] implementation that
/// provides status/error mappings.
///
/// `Self: Sized` means this trait cannot be used as a `dyn` trait object.  Use
/// the concrete [`AsyncCommandExecutor`] directly.
#[allow(async_fn_in_trait)]
pub trait AsyncCommand<S, E, P>
where
    E: Error + Send,
    P: ProcessStatus<S, E> + Send,
    Self: Sized,
{
    /// Creates and spawns a new OS process.
    ///
    /// # Arguments
    ///
    /// * `executable_path` — Path to the executable (e.g. `initdb`, `pg_ctl`).
    /// * `args` — Command-line arguments to pass to the executable.
    /// * `process_type` — The [`ProcessStatus`] value describing this process.
    ///
    /// # Errors
    ///
    /// Returns `E::error_type()` if the process cannot be spawned.
    fn new<A, B>(executable_path: &OsStr, args: A, process_type: P) -> Result<Self, E>
    where
        A: IntoIterator<Item = B>,
        B: AsRef<OsStr>;

    /// Waits for the process to finish, optionally enforcing a deadline.
    ///
    /// Stdout and stderr are captured and forwarded to the [`log`] crate
    /// (at `info` level) in background tasks.
    ///
    /// # Arguments
    ///
    /// * `timeout` — If `Some(duration)`, the process is killed and an error
    ///   is returned if it does not finish within `duration`.  `None` waits
    ///   indefinitely.
    ///
    /// # Returns
    ///
    /// The [`ProcessStatus::status_exit`] value on success.
    ///
    /// # Errors
    ///
    /// Returns [`ProcessStatus::timeout_error`] if the deadline is exceeded.
    /// Returns [`ProcessStatus::error_type`] if the process exits with a
    /// non-zero status.
    /// Returns a wrapped error from [`ProcessStatus::wrap_error`] if waiting
    /// on the process fails.
    async fn execute(&mut self, timeout: Option<Duration>) -> Result<S, E>;
}

/// Concrete implementation of [`AsyncCommand`] built on [`tokio::process`].
///
/// Created through [`AsyncCommand::new`]; the process is spawned immediately
/// and stdout/stderr are piped.  Call [`AsyncCommand::execute`] to wait for
/// completion.
pub struct AsyncCommandExecutor<S, E, P>
where
    S: Send,
    E: Error + Send,
    P: ProcessStatus<S, E>,
    Self: Send,
{
    /// The Tokio command handle (kept alive so the process is not killed on drop).
    _command: tokio::process::Command,
    /// The spawned child process.
    process: Child,
    /// Determines status/error values for this specific process type.
    process_type: P,
    _marker_s: marker::PhantomData<S>,
    _marker_e: marker::PhantomData<E>,
}

impl<S, E, P> AsyncCommandExecutor<S, E, P>
where
    S: Send,
    E: Error + Send,
    P: ProcessStatus<S, E> + Send,
{
    /// Spawns `command` with piped stdout/stderr.
    fn init(command: &mut tokio::process::Command, process_type: &P) -> Result<Child, E> {
        command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|_| process_type.error_type())
    }

    /// Builds a [`tokio::process::Command`] from `executable_path` and `args`.
    fn generate_command<A, B>(executable_path: &OsStr, args: A) -> tokio::process::Command
    where
        A: IntoIterator<Item = B>,
        B: AsRef<OsStr>,
    {
        let mut command = tokio::process::Command::new(executable_path);
        command.args(args);
        command
    }

    /// Reads lines from `data` and forwards them to `sender` until EOF or error.
    async fn handle_output<R: AsyncRead + Unpin>(data: R, sender: Sender<LogOutputData>) {
        let mut lines = BufReader::new(data).lines();
        loop {
            match lines.next_line().await {
                Ok(Some(line)) => {
                    let io_data = LogOutputData {
                        line,
                        log_type: LogType::Info,
                    };
                    if sender.send(io_data).await.is_err() {
                        log::warn!("process output channel closed before stream ended");
                        break;
                    }
                }
                Ok(None) => break,
                Err(e) => {
                    log::error!("Error reading process output: {}", e);
                    break;
                }
            }
        }
    }

    /// Drains `receiver` and writes each line to the [`log`] crate.
    async fn log_output(mut receiver: Receiver<LogOutputData>) {
        while let Some(data) = receiver.recv().await {
            match data.log_type {
                LogType::Info => {
                    log::info!("{}", data.line);
                }
                LogType::Error => {
                    log::error!("{}", data.line);
                }
            }
        }
    }

    /// Awaits the child process exit status.
    async fn run_process(&mut self) -> Result<S, E> {
        let exit_status = self
            .process
            .wait()
            .await
            .map_err(|e| self.process_type.wrap_error(e, None))?;
        if exit_status.success() {
            Ok(self.process_type.status_exit())
        } else {
            Err(self.process_type.error_type())
        }
    }

    /// Waits for the process and drains its output in background tasks.
    async fn command_execution(&mut self) -> Result<S, E> {
        let (sender, receiver) = tokio::sync::mpsc::channel::<LogOutputData>(1000);
        let res = self.run_process().await;
        if let Some(stdout) = self.process.stdout.take() {
            let tx = sender.clone();
            drop(tokio::task::spawn(async move {
                Self::handle_output(stdout, tx).await;
            }));
        }
        if let Some(stderr) = self.process.stderr.take() {
            let tx = sender.clone();
            drop(tokio::task::spawn(async move {
                Self::handle_output(stderr, tx).await;
            }));
        }
        drop(sender);
        drop(tokio::task::spawn(async {
            Self::log_output(receiver).await;
        }));
        res
    }
}

impl<S, E, P> AsyncCommand<S, E, P> for AsyncCommandExecutor<S, E, P>
where
    S: Send,
    E: Error + Send,
    P: ProcessStatus<S, E> + Send,
{
    fn new<A, B>(executable_path: &OsStr, args: A, process_type: P) -> Result<Self, E>
    where
        A: IntoIterator<Item = B>,
        B: AsRef<OsStr>,
    {
        let mut _command = Self::generate_command(executable_path, args);
        let process = Self::init(&mut _command, &process_type)?;
        Ok(AsyncCommandExecutor {
            _command,
            process,
            process_type,
            _marker_s: Default::default(),
            _marker_e: Default::default(),
        })
    }

    async fn execute(&mut self, timeout: Option<Duration>) -> Result<S, E> {
        match timeout {
            None => self.command_execution().await,
            Some(duration) => tokio::time::timeout(duration, self.command_execution())
                .await
                .map_err(|_| self.process_type.timeout_error())?,
        }
    }
}
