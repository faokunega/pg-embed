//!
//! Process command creation and execution
//!
use std::cell::Cell;
use std::error::Error;
use std::ffi::OsStr;
use std::io::ErrorKind;
use std::process::Stdio;

use async_trait::async_trait;
use futures::TryFutureExt;
use log;
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader, Lines};
use tokio::process::{Child, ChildStdout};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::task::JoinHandle;
use tokio::time::Duration;

///
/// Output logging type
///
pub enum LogType {
    Info,
    Error,
}

///
/// Child process status
///
pub trait ProcessStatus<T, E: Error> {
    /// process entry status
    fn status_entry(&self) -> T;
    /// process exit status
    fn status_exit(&self) -> T;
    /// process error type
    fn error_type(&self) -> E;
    /// wrap error
    fn wrap_error(&self, error: dyn std::error::Error) -> E;
}

///
/// Logging data
///
pub struct LogOutputData {
    line: String,
    log_type: LogType,
}

///
/// Async command trait
///
#[async_trait]
pub trait AsyncCommand<S, E: Error, T> {
    ///
    /// Create a new async command
    ///
    fn new<ARGS, ARG>(executable_path: &OsStr, args: ARGS, process_type: T) -> Result<Self, E>
    where
        ARGS: IntoIterator<Item = ARG>,
        ARG: AsRef<OsStr>;
    ///
    /// Execute command
    ///
    /// When timeout is Some(duration) the process execution will be timed out after duration,
    /// if set to None the process execution will not be timed out.
    ///
    async fn execute(&self, timeout: Option<Duration>) -> Result<S, E>;
}

///
/// Process command
///
pub struct AsyncCommandExecutor<S, E: Error, T: ProcessStatus<S, E>> {
    /// Process command
    command: tokio::process::Command,
    /// Process child
    process: Child,
    /// Process type
    process_type: T,
}

impl<S, E, T> AsyncCommandExecutor<S, E, T>
where
    E: Error,
    T: ProcessStatus<S, E>,
{
    /// Initialize command
    fn init(command: &mut tokio::process::Command, process_type: &T) -> Result<Child, E> {
        command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| process_type.error_type(e))
    }

    /// Generate a command
    fn generate_command<ARGS, ARG>(executable_path: &OsStr, args: ARGS) -> tokio::process::Command
    where
        ARGS: IntoIterator<Item = ARG>,
        ARG: AsRef<OsStr>,
    {
        let mut command = tokio::process::Command::new(executable_path);
        command.args(args);
        command
    }

    /// Handle process output
    async fn handle_output<READ: AsyncRead>(
        &self,
        data: READ,
        sender: Sender<LogOutputData>,
    ) -> Result<(), E> {
        let mut lines = BufReader::new(data).lines();
        while let Some(line) = lines.next_line().await? {
            let io_data = LogOutputData {
                line,
                log_type: LogType::Info,
            };
            sender
                .send(io_data)
                .map_err(|e| self.process_type.wrap_error(e))
                .await?;
        }
        Ok(())
    }

    /// Log process output
    async fn log_output(mut receiver: Receiver<LogOutputData>) -> () {
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

    /// Run process
    async fn run_process(&mut self) -> Result<S, E> {
        let exit_status = self
            .process
            .wait()
            .await
            .map_err(|e| self.process_type.wrap_error(e))?;
        if exit_status.success() {
            Ok(self.process_type.status_exit())
        } else {
            Err(self.process_type.error_type())
        }
    }
}

#[async_trait]
impl<S, E, T> AsyncCommand<S, E, T> for AsyncCommandExecutor<S, E, T>
where
    E: Error,
    T: ProcessStatus<S, E>,
{
    fn new<ARGS, ARG>(executable_path: &OsStr, args: ARGS, process_type: T) -> Result<Self, E> {
        let mut command = Self::generate_command(executable_path, args);
        let process = Self::init(&mut command, &process_type)?;
        Ok(AsyncCommandExecutor {
            command,
            process,
            process_type,
        })
    }

    async fn execute(&mut self, timeout: Option<Duration>) -> Result<S, E> {
        let (sender, mut receiver) = tokio::sync::mpsc::channel::<LogOutputData>(1000);
        {
            let tx = sender.clone();
            let stdout = self.process.stdout.take().unwrap();
            tokio::spawn(async move {
                self.handle_output(stdout, tx).await?;
            });
        }
        {
            let stderr = self.process.stderr.take().unwrap();
            tokio::spawn(async move {
                self.handle_output(stderr, sender).await?;
            });
        }
        Self::log_output(receiver).await;
        self.run_process().await
    }
}
