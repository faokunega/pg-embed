use futures::future::BoxFuture;
use futures::{AsyncWriteExt, TryFutureExt};
use std::process::{Command, Child};
use crate::fetch;
use crate::errors::PgEmbedError;

///
/// Database settings
///
pub struct PgSettings {
    /// postgresql executables directory
    pub executables_dir: String,
    /// postgresql database directory
    pub database_dir: String,
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
pub struct PgEmbed {
    pub pg_settings: PgSettings,
    pub fetch_settings: fetch::FetchSettings,
    pub process: Option<Child>,
}

impl Drop for PgEmbed {
    fn drop(&mut self) {
        self.process.as_mut().map(|p| p.kill());
    }
}

impl PgEmbed {
    pub fn new(pg_settings: PgSettings, fetch_settings: fetch::FetchSettings) -> Self {
        PgEmbed {
            pg_settings,
            fetch_settings,
            process: None,
        }
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
    /// Returns the child process `Ok(Child)` on success, otherwise returns an error.
    ///
    pub async fn init_db(&self) -> Result<Child, PgEmbedError> {
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
        Ok(process)
    }

    ///
    /// Start postgresql database
    ///
    /// Returns `Ok(())` on success, otherwise returns an error.
    ///
    pub async fn start_db(&mut self) -> Result<(), PgEmbedError> {
        let pg_ctl_executable = format!("{}/bin/pg_ctl", &self.pg_settings.executables_dir);
        let mut process = Command::new(
            pg_ctl_executable,
        )
            .args(&[
                "start", "-w", "-D", &self.pg_settings.database_dir,
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
        let mut file = async_std::fs::File::create(&file_path).map_err(|e| PgEmbedError::WriteFileError(e)).await?;
        let _ = file
            .write(&self.pg_settings.password.as_bytes()).map_err(|e| PgEmbedError::WriteFileError(e))
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod postgres_tests {
    use super::*;

    // #[test]
    // fn postgres_start() -> anyhow::Result<()> {
    //     start_db("data/db")
    // }
    //
    // #[async_std::test]
    // async fn password_file_creation() -> anyhow::Result<()> {
    //     create_password_file(
    //         "data", "password",
    //     )
    //         .await
    // }
    //
    // #[test]
    // fn database_initialization() -> anyhow::Result<()> {
    //     init_db("data/db", "postgres")
    // }
}
