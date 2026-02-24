//! Factories for the three pg_ctl / initdb command executors.
//!
//! Each function in [`PgCommand`] constructs an [`AsyncCommandExecutor`] that
//! is ready to run but has not yet been awaited.  Callers obtain the executor,
//! then call [`crate::command_executor::AsyncCommand::execute`] to actually
//! run the command.

use std::path::Path;

use crate::command_executor::{AsyncCommand, AsyncCommandExecutor};
use crate::pg_enums::{PgAuthMethod, PgProcessType, PgServerStatus};
use crate::pg_errors::Error;
use crate::pg_errors::Result;

/// Factories for the three PostgreSQL lifecycle commands.
pub struct PgCommand {}

impl PgCommand {
    /// Creates an [`AsyncCommandExecutor`] that runs `initdb` to initialise a
    /// new database cluster.
    ///
    /// # Arguments
    ///
    /// * `init_db_exe` — Path to the `initdb` binary.
    /// * `database_dir` — Target directory for the new cluster.
    /// * `pw_file_path` — Path to the password file created by
    ///   [`crate::pg_access::PgAccess::create_password_file`].
    /// * `user` — Name of the initial superuser.
    /// * `auth_method` — Authentication method written to `pg_hba.conf`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidPgUrl`] if any of the path arguments cannot be
    /// converted to a UTF-8 string (required for the `--pwfile=` argument
    /// format).
    /// Returns [`Error::PgInitFailure`] if the process cannot be spawned.
    pub fn init_db_executor(
        init_db_exe: &Path,
        database_dir: &Path,
        pw_file_path: &Path,
        user: &str,
        auth_method: &PgAuthMethod,
    ) -> Result<AsyncCommandExecutor<PgServerStatus, Error, PgProcessType>> {
        let init_db_executable = init_db_exe.as_os_str();
        let pw_file_str = pw_file_path
            .to_str()
            .ok_or(Error::InvalidPgUrl)?;
        let password_file_arg = format!("--pwfile={}", pw_file_str);
        let auth_host = match auth_method {
            PgAuthMethod::Plain => "password",
            PgAuthMethod::MD5 => "md5",
            PgAuthMethod::ScramSha256 => "scram-sha-256",
        };
        let db_dir_str = database_dir.to_str().ok_or(Error::InvalidPgUrl)?;
        let args = [
            "-A",
            auth_host,
            "-U",
            user,
            // The postgres-tokio driver uses utf8 encoding, however on windows
            // if -E is not specified WIN1252 encoding is chosen by default
            // which can lead to encoding errors like this:
            //
            // ERROR: character with byte sequence 0xe0 0xab 0x87 in encoding
            // "UTF8" has no equivalent in encoding "WIN1252"
            "-E=UTF8",
            "-D",
            db_dir_str,
            &password_file_arg,
        ];

        AsyncCommandExecutor::<PgServerStatus, Error, PgProcessType>::new(
            init_db_executable,
            args,
            PgProcessType::InitDb,
        )
    }

    /// Creates an [`AsyncCommandExecutor`] that runs `pg_ctl start`.
    ///
    /// # Arguments
    ///
    /// * `pg_ctl_exe` — Path to the `pg_ctl` binary.
    /// * `database_dir` — The cluster directory passed to `pg_ctl -D`.
    /// * `port` — TCP port PostgreSQL should listen on.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidPgUrl`] if `database_dir` is not valid UTF-8.
    /// Returns [`Error::PgStartFailure`] if the process cannot be spawned.
    pub fn start_db_executor(
        pg_ctl_exe: &Path,
        database_dir: &Path,
        port: &u16,
    ) -> Result<AsyncCommandExecutor<PgServerStatus, Error, PgProcessType>> {
        let pg_ctl_executable = pg_ctl_exe.as_os_str();
        let port_arg = format!("-F -p {}", port);
        let db_dir_str = database_dir.to_str().ok_or(Error::InvalidPgUrl)?;
        let args = ["-o", &port_arg, "start", "-w", "-D", db_dir_str];
        AsyncCommandExecutor::<PgServerStatus, Error, PgProcessType>::new(
            pg_ctl_executable,
            args,
            PgProcessType::StartDb,
        )
    }

    /// Creates an [`AsyncCommandExecutor`] that runs `pg_ctl stop`.
    ///
    /// # Arguments
    ///
    /// * `pg_ctl_exe` — Path to the `pg_ctl` binary.
    /// * `database_dir` — The cluster directory passed to `pg_ctl -D`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidPgUrl`] if `database_dir` is not valid UTF-8.
    /// Returns [`Error::PgStopFailure`] if the process cannot be spawned.
    pub fn stop_db_executor(
        pg_ctl_exe: &Path,
        database_dir: &Path,
    ) -> Result<AsyncCommandExecutor<PgServerStatus, Error, PgProcessType>> {
        let pg_ctl_executable = pg_ctl_exe.as_os_str();
        let db_dir_str = database_dir.to_str().ok_or(Error::InvalidPgUrl)?;
        let args = ["stop", "-w", "-D", db_dir_str];
        AsyncCommandExecutor::<PgServerStatus, Error, PgProcessType>::new(
            pg_ctl_executable,
            args,
            PgProcessType::StopDb,
        )
    }
}
