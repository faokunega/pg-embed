//!
//! Postgresql server
//!
//! Start, stop, initialize the postgresql server.
//! Create database clusters and databases.
//!
use futures::future::BoxFuture;
use futures::{TryFutureExt};
use std::process::{Command, Child};
use crate::fetch;
use crate::errors::PgEmbedError;
#[cfg(any(feature = "rt_tokio", feature = "rt_tokio_migrate"))]
use tokio::io::AsyncWriteExt;
use crate::errors::PgEmbedError::PgCleanUpFailure;
#[cfg(feature = "rt_tokio_migrate")]
use sqlx_tokio::{Connection, PgConnection, Postgres};
use std::borrow::{BorrowMut, Borrow};
use std::time::Duration;
#[cfg(any(feature = "rt_tokio", feature = "rt_tokio_migrate"))]
use tokio::time::sleep;
#[cfg(feature = "rt_tokio_migrate")]
use sqlx_tokio::migrate::{Migrator, MigrateDatabase};
use std::path::PathBuf;

///
/// Database settings
///
pub struct PgSettings {
    /// postgresql executables directory
    pub executables_dir: PathBuf,
    /// postgresql database directory
    pub database_dir: PathBuf,
    /// postgresql port
    pub port: i16,
    /// postgresql user name
    pub user: String,
    /// authentication
    pub auth_method: PgAuthMethod,
    /// persist database
    pub persistent: bool,
    /// duration to wait for postgresql process to start
    pub start_timeout: Duration,
    /// migrations folder
    /// sql script files to execute on migrate
    pub migration_dir: Option<PathBuf>,
}

///
/// Postgresql password authentication method
///
/// Choose between plain password, md5 or scram_sha_256 authentication.
/// The plain password is encapsulated by PgAuthMethod::Plain.
/// Md5 and scram_sha_256 authentication are only available on postgresql versions >= 11,
/// and are provided through the postgresql password file
///
pub enum PgAuthMethod {
    // plain password encapsulated in enum as string
    Plain(String),
    // md5 password hash, provided through password file
    MD5,
    // scram_sha_256 password hash, provided through password file
    ScramSha256,
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
    pub db_uri: String,
}

impl Drop for PgEmbed {
    fn drop(&mut self) {
        let _ = &self.stop_db();
        if !&self.pg_settings.persistent {
            let _ = &self.clean();
        }
    }
}

impl PgEmbed {
    ///
    /// Create a new PgEmbed instance
    ///
    pub fn new(pg_settings: PgSettings, fetch_settings: fetch::FetchSettings) -> Self {
        let password = match &pg_settings.auth_method {
            PgAuthMethod::Plain(pass) => {
                pass.as_str()
            }
            PgAuthMethod::MD5 => {
                ""
            }
            PgAuthMethod::ScramSha256 => {
                ""
            }
        };
        let db_uri = format!(
            "postgres://{}:{}@localhost:{}",
            &pg_settings.user,
            &password,
            &pg_settings.port
        );
        PgEmbed {
            pg_settings,
            fetch_settings,
            process: None,
            db_uri,
        }
    }

    ///
    /// Clean up created files and directories.
    ///
    /// Remove created directories containing the postgresql executables, the database and the password file.
    ///
    pub fn clean(&self) -> Result<(), PgEmbedError> {
        let exec_dir = self.pg_settings.executables_dir.to_str()?;
        let bin_dir = format!("{}/bin", exec_dir);
        let lib_dir = format!("{}/lib", exec_dir);
        let share_dir = format!("{}/share", exec_dir);
        let pw_file = format!("{}/pwfile", exec_dir);
        // not using tokio::fs async methods because clean() is called on drop
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
        let database_path = self.pg_settings.database_dir.as_path();
        if !database_path.is_dir() {
            let init_db_executable = format!("{}/bin/initdb", &self.pg_settings.executables_dir.to_str()?);
            let mut init_db_command = Command::new(init_db_executable);
            // determine which authentication method to use
            let process =
                match &self.pg_settings.auth_method {
                    PgAuthMethod::Plain(password) => {
                        init_db_command.args(&[])
                    }
                    PgAuthMethod::MD5 => {
                        let password_file_arg = format!("--pwfile={}/pwfile", &self.pg_settings.executables_dir.to_str()?);
                        init_db_command.args(&[])
                    }
                    PgAuthMethod::ScramSha256 => {
                        let password_file_arg = format!("--pwfile={}/pwfile", &self.pg_settings.executables_dir.to_str()?);
                        init_db_command.args(&[])
                    }
                };
            process.spawn().map_err(|e| PgEmbedError::PgInitFailure(e))?;
            sleep(self.pg_settings.start_timeout).await;
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
        let pg_ctl_executable = format!("{}/bin/pg_ctl", &self.pg_settings.executables_dir.to_str().unwrap());
        let port_arg = format!("-F -p {}", &self.pg_settings.port.to_string());
        let mut process = Command::new(
            pg_ctl_executable,
        )
            .args(&[
                "-o", &port_arg, "start", "-w", "-D", &self.pg_settings.database_dir.to_str().unwrap()
            ])
            .spawn().map_err(|e| PgEmbedError::PgStartFailure(e))?;
        self.process = Some(process);
        sleep(self.pg_settings.start_timeout).await;
        Ok(())
    }

    ///
    /// Stop postgresql database
    ///
    /// Returns `Ok(())` on success, otherwise returns an error.
    ///
    pub fn stop_db(&mut self) -> Result<(), PgEmbedError> {
        let pg_ctl_executable = format!("{}/bin/pg_ctl", &self.pg_settings.executables_dir.to_str().unwrap());
        let mut process = Command::new(
            pg_ctl_executable,
        )
            .args(&[
                "stop", "-w", "-D", &self.pg_settings.database_dir.to_str().unwrap(),
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
        let mut file_path = self.pg_settings.executables_dir.clone();
        file_path.push("pwfile");
        let mut file: tokio::fs::File = tokio::fs::File::create(&file_path.as_path()).map_err(|e| PgEmbedError::WriteFileError(e)).await?;
        let _ = file
            .write(&self.pg_settings.password.as_bytes()).map_err(|e| PgEmbedError::WriteFileError(e))
            .await?;
        Ok(())
    }

    ///
    /// Create a database
    ///
    #[cfg(any(feature = "rt_tokio_migrate", feature = "rt_async_std_migrate", feature = "rt_actix_migrate"))]
    pub async fn create_database(&self, db_name: &str) -> Result<(), PgEmbedError> {
        Postgres::create_database(&self.full_db_uri(db_name)).await?;
        Ok(())
    }

    ///
    /// Drop a database
    ///
    #[cfg(any(feature = "rt_tokio_migrate", feature = "rt_async_std_migrate", feature = "rt_actix_migrate"))]
    pub async fn drop_database(&self, db_name: &str) -> Result<(), PgEmbedError> {
        Postgres::drop_database(&self.full_db_uri(db_name)).await?;
        Ok(())
    }

    ///
    /// Check database existance
    ///
    #[cfg(any(feature = "rt_tokio_migrate", feature = "rt_async_std_migrate", feature = "rt_actix_migrate"))]
    pub async fn database_exists(&self, db_name: &str) -> Result<bool, PgEmbedError> {
        let result = Postgres::database_exists(&self.full_db_uri(db_name)).await?;
        Ok(result)
    }

    ///
    /// The full database uri
    ///
    /// (*postgres://{username}:{password}@localhost:{port}/{db_name}*)
    ///
    pub fn full_db_uri(&self, db_name: &str) -> String {
        format!("{}/{}", &self.db_uri, db_name)
    }

    ///
    /// Run migrations
    ///
    #[cfg(any(feature = "rt_tokio_migrate", feature = "rt_async_std_migrate", feature = "rt_actix_migrate"))]
    pub async fn migrate(&self, db_name: &str) -> Result<(), PgEmbedError> {
        if let Some(migration_dir) = &self.pg_settings.migration_dir {
            let m = Migrator::new(std::path::Path::new(migration_dir)).await?;
            let pool = sqlx_tokio::postgres::PgPoolOptions::new().connect(&self.full_db_uri(db_name)).await?;
            m.run(&pool).await?;
        }
        Ok(())
    }
}