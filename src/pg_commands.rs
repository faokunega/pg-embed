//!
//! Create postgres command executor
//!
//! Command executors for initdb, pg_ctl start, pg_ctl stop
//!
use std::path::PathBuf;

use crate::command_executor::{AsyncCommand, AsyncCommandExecutor};
use crate::pg_enums::{PgAuthMethod, PgProcessType, PgServerStatus};
use crate::pg_errors::PgEmbedError;
use crate::pg_types::PgResult;

///
/// Postgres command executors
///
pub struct PgCommand {}

impl PgCommand {
    ///
    /// Create initdb command
    ///
    pub fn init_db_executor(
        init_db_exe: &PathBuf,
        database_dir: &PathBuf,
        pw_file_path: &PathBuf,
        user: &str,
        auth_method: &PgAuthMethod,
    ) -> PgResult<AsyncCommandExecutor<PgServerStatus, PgEmbedError, PgProcessType>> {
        let init_db_executable = init_db_exe.as_os_str();
        let password_file_arg = format!("--pwfile={}", pw_file_path.to_str().unwrap());
        let auth_host = match auth_method {
            PgAuthMethod::Plain => "password",
            PgAuthMethod::MD5 => "md5",
            PgAuthMethod::ScramSha256 => "scram-sha-256",
        };
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
            database_dir.to_str().unwrap(),
            &password_file_arg,
        ];

        let command_executor =
            AsyncCommandExecutor::<PgServerStatus, PgEmbedError, PgProcessType>::new(
                init_db_executable,
                args,
                PgProcessType::InitDb,
            )?;

        Ok(command_executor)
    }

    ///
    /// Create pg_ctl start command
    ///
    pub fn start_db_executor(
        pg_ctl_exe: &PathBuf,
        database_dir: &PathBuf,
        port: &u16,
    ) -> PgResult<AsyncCommandExecutor<PgServerStatus, PgEmbedError, PgProcessType>> {
        let pg_ctl_executable = pg_ctl_exe.as_os_str();
        let port_arg = format!("-F -p {}", port.to_string());
        let args = [
            "-o",
            &port_arg,
            "start",
            "-w",
            "-D",
            database_dir.to_str().unwrap(),
        ];
        let command_executor =
            AsyncCommandExecutor::<PgServerStatus, PgEmbedError, PgProcessType>::new(
                pg_ctl_executable,
                args,
                PgProcessType::StartDb,
            )?;

        Ok(command_executor)
    }

    ///
    /// Create pg_ctl stop command
    ///
    pub fn stop_db_executor(
        pg_ctl_exe: &PathBuf,
        database_dir: &PathBuf,
    ) -> PgResult<AsyncCommandExecutor<PgServerStatus, PgEmbedError, PgProcessType>> {
        let pg_ctl_executable = pg_ctl_exe.as_os_str();
        let args = ["stop", "-w", "-D", database_dir.to_str().unwrap()];
        let command_executor =
            AsyncCommandExecutor::<PgServerStatus, PgEmbedError, PgProcessType>::new(
                pg_ctl_executable,
                args,
                PgProcessType::StopDb,
            )?;

        Ok(command_executor)
    }
}
