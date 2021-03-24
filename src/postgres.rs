use futures::future::BoxFuture;
use futures::{TryFutureExt};
use std::process::{Command, Child};
use crate::fetch;
use crate::errors::PgEmbedError;
use tokio::io::AsyncWriteExt;
use crate::errors::PgEmbedError::PgCleanUpFailure;

///
/// Database settings
///
pub struct PgSettings {
    /// postgresql executables directory
    pub executables_dir: String,
    /// postgresql database directory
    pub database_dir: String,
    /// postgresql port
    pub port: i16,
    /// postgresql user name
    pub user: String,
    /// postgresql password
    pub password: String,
    /// persist database
    pub persistent: bool,
}

///
/// Embedded postgresql database
///
/// If the PgEmbed instance is dropped / goes out of scope and postgresql is still
/// running, the postgresql process will be killed and depending on the [PgSettings::persistent] setting,
/// file and directories will be cleaned up.
///
pub struct PgEmbed {
    /// Postgresql settings
    pub pg_settings: PgSettings,
    /// Download settings
    pub fetch_settings: fetch::FetchSettings,
    ///
    /// The postgresql process
    ///
    /// `Some(process)` if process is running, otherwise `None`
    ///
    pub process: Option<Child>,
}

impl Drop for PgEmbed {
    fn drop(&mut self) {
        &self.process.as_mut().map(|p| p.kill());
        if !&self.pg_settings.persistent {
            &self.clean();
        }
    }
}

impl PgEmbed {
    ///
    /// Create a new PgEmbed instance
    ///
    pub fn new(pg_settings: PgSettings, fetch_settings: fetch::FetchSettings) -> Self {
        PgEmbed {
            pg_settings,
            fetch_settings,
            process: None,
        }
    }

    ///
    /// Clean up created files and directories.
    ///
    /// Remove created directories containing the postgresql executables, the database and the password file.
    ///
    pub fn clean(&self) -> Result<(), PgEmbedError> {
        let bin_dir = format!("{}/bin", &self.pg_settings.executables_dir);
        let lib_dir = format!("{}/lib", &self.pg_settings.executables_dir);
        let share_dir = format!("{}/share", &self.pg_settings.executables_dir);
        let pw_file = format!("{}/pwfile", &self.pg_settings.executables_dir);
        std::fs::remove_dir_all(&self.pg_settings.database_dir).map_err(|e| PgCleanUpFailure(e))?;
        std::fs::remove_dir_all(bin_dir).map_err(|e| PgCleanUpFailure(e))?;
        std::fs::remove_dir_all(lib_dir).map_err(|e| PgCleanUpFailure(e))?;
        std::fs::remove_dir_all(share_dir).map_err(|e| PgCleanUpFailure(e))?;
        std::fs::remove_file(pw_file).map_err(|e| PgCleanUpFailure(e))?;
        Ok(())
    }

    ///
    /// Setup postgresql for execution
    ///
    /// Download, unpack, create password file and database
    ///
    pub async fn setup(&self) -> Result<(), PgEmbedError> {
        &self.aquire_postgres().await?;
        &self.create_password_file().await?;
        &self.init_db().await?;
        Ok(())
    }

    ///
    /// Download and unpack postgres binaries
    ///
    pub async fn aquire_postgres(&self) -> Result<(), PgEmbedError> {
        let pg_file = fetch::fetch_postgres(&self.fetch_settings, &self.pg_settings.executables_dir).await?;
        fetch::unpack_postgres(&pg_file, &self.pg_settings.executables_dir).await
    }

    ///
    /// Initialize postgresql database
    ///
    /// Returns `Ok(bool)` on success (false if the database directory already exists, true if it needed to be created),
    /// otherwise returns an error.
    ///
    pub async fn init_db(&self) -> Result<bool, PgEmbedError> {
        let database_path = std::path::Path::new(&self.pg_settings.database_dir);
        if !database_path.is_dir() {
            let init_db_executable = format!("{}/bin/initdb", &self.pg_settings.executables_dir);
            let password_file_arg = format!("--pwfile={}/pwfile", &self.pg_settings.executables_dir);
            let process = Command::new(
                init_db_executable,
            )
                .args(&[
                    "-A",
                    &self.pg_settings.password,
                    "-U",
                    &self.pg_settings.user,
                    "-D",
                    &self.pg_settings.database_dir,
                    &password_file_arg,
                ])
                .spawn().map_err(|e| PgEmbedError::PgInitFailure(e))?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    ///
    /// Start postgresql database
    ///
    /// Returns `Ok(())` on success, otherwise returns an error.
    ///
    pub async fn start_db(&mut self) -> Result<(), PgEmbedError> {
        let pg_ctl_executable = format!("{}/bin/pg_ctl", &self.pg_settings.executables_dir);
        let port_arg = format!("-F -p {}", &self.pg_settings.port.to_string());
        let mut process = Command::new(
            pg_ctl_executable,
        )
            .args(&[
                "-o", &port_arg, "start", "-w", "-D", &self.pg_settings.database_dir
            ])
            .spawn().map_err(|e| PgEmbedError::PgStartFailure(e))?;
        self.process = Some(process);
        Ok(())
    }

    ///
    /// Stop postgresql database
    ///
    /// Returns `Ok(())` on success, otherwise returns an error.
    ///
    pub async fn stop_db(&mut self) -> Result<(), PgEmbedError> {
        let pg_ctl_executable = format!("{}/bin/pg_ctl", &self.pg_settings.executables_dir);
        let mut process = Command::new(
            pg_ctl_executable,
        )
            .args(&[
                "stop", "-w", "-D", &self.pg_settings.database_dir,
            ])
            .spawn().map_err(|e| PgEmbedError::PgStopFailure(e))?;

        match process.try_wait() {
            Ok(Some(status)) => {
                println!("postgresql stopped");
                self.process = None;
                Ok(())
            }
            Ok(None) => {
                println!("... waiting for postgresql to stop");
                let res = process.wait();
                println!("result: {:?}", res);
                Ok(())
            }
            Err(e) => Err(PgEmbedError::PgStopFailure(e)),
        }
    }

    ///
    /// Create a database password file
    ///
    /// Returns `Ok(())` on success, otherwise returns an error.
    ///
    pub async fn create_password_file(&self) -> Result<(), PgEmbedError> {
        let file_path = format!(
            "{}/{}",
            &self.pg_settings.executables_dir, "pwfile"
        );
        let mut file: tokio::fs::File = tokio::fs::File::create(&file_path).map_err(|e| PgEmbedError::WriteFileError(e)).await?;
        let _ = file
            .write(&self.pg_settings.password.as_bytes()).map_err(|e| PgEmbedError::WriteFileError(e))
            .await?;
        Ok(())
    }
}