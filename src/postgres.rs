//!
//! Postgresql server
//!
//! Start, stop, initialize the postgresql server.
//! Create database clusters and databases.
//!
use futures::{TryFutureExt};
use std::process::{Command, Stdio};
use crate::pg_fetch;
// these cfg feature settings for PgEmbedError are really convoluted, but getting syntax errors otherwise
#[cfg(not(any(feature = "rt_tokio_migrate", feature = "rt_async_std", feature = "rt_async_std_migrate", feature = "rt_actix", feature = "rt_actix_migrate")))]
use crate::errors::errors_tokio::PgEmbedError;
#[cfg(not(any(feature = "rt_tokio", feature = "rt_async_std", feature = "rt_async_std_migrate", feature = "rt_actix", feature = "rt_actix_migrate")))]
use crate::errors::errors_tokio_migrate::PgEmbedError;
#[cfg(not(any(feature = "rt_tokio", feature = "rt_tokio_migrate", feature = "rt_async_std_migrate", feature = "rt_actix", feature = "rt_actix_migrate")))]
use crate::errors::errors_async_std::PgEmbedError;
#[cfg(not(any(feature = "rt_tokio", feature = "rt_tokio_migrate", feature = "rt_async_std", feature = "rt_actix", feature = "rt_actix_migrate")))]
use crate::errors::errors_async_std_migrate::PgEmbedError;
#[cfg(not(any(feature = "rt_tokio", feature = "rt_tokio_migrate", feature = "rt_async_std", feature = "rt_async_std_migrate", feature = "rt_actix_migrate")))]
use crate::errors::errors_actix::PgEmbedError;
#[cfg(not(any(feature = "rt_tokio", feature = "rt_tokio_migrate", feature = "rt_async_std", feature = "rt_async_std_migrate", feature = "rt_actix")))]
use crate::errors::errors_actix_migrate::PgEmbedError;
use crate::errors::errors_common::PgEmbedError;
#[cfg(any(feature = "rt_tokio", feature = "rt_tokio_migrate"))]
use tokio::io::AsyncWriteExt;
#[cfg(feature = "rt_tokio_migrate")]
use sqlx_tokio::{Postgres};
use std::time::Duration;
#[cfg(feature = "rt_tokio_migrate")]
use sqlx_tokio::migrate::{Migrator, MigrateDatabase};
use std::path::PathBuf;
use process_control::ChildExt;
use process_control::Timeout;
use std::io;
use io::{Error, ErrorKind};
use log::{info, error};
use crate::pg_access::PgAccess;

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
    /// postgresql password
    pub password: String,
    /// authentication
    pub auth_method: PgAuthMethod,
    /// persist database
    pub persistent: bool,
    /// duration to wait before terminating process execution
    /// pg_ctl start/stop and initdb timeout
    pub timeout: Duration,
    /// migrations folder
    /// sql script files to execute on migrate
    pub migration_dir: Option<PathBuf>,
}

///
/// Postgresql authentication method
///
/// Choose between plain password, md5 or scram_sha_256 authentication.
/// Scram_sha_256 authentication is only available on postgresql versions >= 11
///
pub enum PgAuthMethod {
    /// plain-text
    Plain,
    /// md5
    MD5,
    /// scram_sha_256
    ScramSha256,
}

///
/// Postgresql server status
///
#[derive(PartialEq)]
pub enum PgServerStatus {
    /// Postgres uninitialized
    Uninitialized,
    /// Initialization process running
    Initializing,
    /// Initialization process finished
    Initialized,
    /// Postgres server process starting
    Starting,
    /// Postgres server process started
    Started,
    /// Postgres server process stopping
    Stopping,
    /// Postgres server process stopped
    Stopped,
    /// Postgres failure
    Failure,
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
    pub fetch_settings: pg_fetch::PgFetchSettings,
    /// Database uri `postgres://{username}:{password}@localhost:{port}`
    pub db_uri: String,
    /// Postgres server status
    pub server_status: PgServerStatus,
    /// Postgres files access
    pub pg_access: PgAccess,
}

impl Drop for PgEmbed {
    fn drop(&mut self) {
        if self.server_status != PgServerStatus::Stopped {
            let _ = &self.stop_db();
        }
        if !&self.pg_settings.persistent {
            let _ = &self.pg_access.clean();
        }
    }
}

impl PgEmbed {
    ///
    /// Create a new PgEmbed instance
    ///
    pub async fn new(pg_settings: PgSettings, fetch_settings: pg_fetch::PgFetchSettings) -> Result<Self, PgEmbedError> {
        let password: &str = &pg_settings.password;
        let db_uri = format!(
            "postgres://{}:{}@localhost:{}",
            &pg_settings.user,
            &password,
            &pg_settings.port
        );
        let pg_access = PgAccess::new(&fetch_settings, &pg_settings.database_dir).await?;
        Ok(
            PgEmbed {
                pg_settings,
                fetch_settings,
                db_uri,
                server_status: PgServerStatus::Uninitialized,
                pg_access,
            }
        )
    }

    ///
    /// Setup postgresql for execution
    ///
    /// Download, unpack, create password file and database
    ///
    pub async fn setup(&mut self) -> Result<(), PgEmbedError> {
        &self.aquire_postgres().await?;
        self.pg_access.create_password_file(self.pg_settings.password.as_bytes()).await?;
        &self.init_db().await?;
        Ok(())
    }

    ///
    /// Download and unpack postgres binaries
    ///
    pub async fn aquire_postgres(&self) -> Result<(), PgEmbedError> {
        let pg_file = pg_fetch::fetch_postgres(&self.fetch_settings, &self.pg_settings.executables_dir).await?;
        pg_fetch::unpack_postgres(&pg_file, &self.pg_settings.executables_dir).await
    }

    ///
    /// Initialize postgresql database
    ///
    /// Returns `Ok(bool)` on success (false if the database directory already exists, true if it needed to be created),
    /// otherwise returns an error.
    ///
    pub async fn init_db(&mut self) -> Result<bool, PgEmbedError> {
        self.server_status = PgServerStatus::Initializing;
        let database_path = self.pg_settings.database_dir.as_path();
        if !database_path.is_dir() {
            let init_db_executable = self.pg_access.init_db_exe.to_str().unwrap();
            let password_file_arg = format!("--pwfile={}/pwfile", &self.pg_access.cache_dir.to_str().unwrap());
            // determine which authentication method to use
            let auth_host =
                match &self.pg_settings.auth_method {
                    PgAuthMethod::Plain => {
                        "password"
                    }
                    PgAuthMethod::MD5 => {
                        "md5"
                    }
                    PgAuthMethod::ScramSha256 => {
                        "scram-sha-256"
                    }
                };

            let mut process = Command::new(init_db_executable)
                .args(&[
                    "-A",
                    auth_host,
                    "-U",
                    &self.pg_settings.user,
                    "-D",
                    &self.pg_settings.database_dir.to_str().unwrap(),
                    &password_file_arg,
                ])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|e| PgEmbedError::PgInitFailure(e))?;

            let exit_status = process
                .with_output_timeout(self.pg_settings.timeout)
                .strict_errors()
                .terminating()
                .wait()
                .map_err(|e| PgEmbedError::PgInitFailure(e))?
                .ok_or_else(|| PgEmbedError::PgInitFailure(Error::new(ErrorKind::TimedOut, "Postgresql initialization command timed out")))?;

            if exit_status.status.success() {
                info!(String::from_utf8(exit_status.stdout).unwrap());
                self.server_status = PgServerStatus::Initialized;
                Ok(true)
            } else {
                error!(String::from_utf8(exit_status.stderr).unwrap());
                self.server_status = PgServerStatus::Failure;
                Err(PgEmbedError::PgInitFailure(Error::new(ErrorKind::Other, format!("Postgresql initialization command failed with {}", exit_status.status))))
            }
        } else {
            self.server_status = PgServerStatus::Failure;
            Ok(false)
        }
    }

    ///
    /// Start postgresql database
    ///
    /// Returns `Ok(())` on success, otherwise returns an error.
    ///
    pub async fn start_db(&mut self) -> Result<(), PgEmbedError> {
        self.server_status = PgServerStatus::Starting;
        let pg_ctl_executable = self.pg_access.pg_ctl_exe.to_str().unwrap();
        let port_arg = format!("-F -p {}", &self.pg_settings.port.to_string());
        // TODO: somehow the standard output of this command can not be piped, if piped it does not terminate. Find a solution!
        let mut process = Command::new(
            pg_ctl_executable,
        )
            .args(&[
                "-o", &port_arg, "start", "-w", "-D", &self.pg_settings.database_dir.to_str().unwrap()
            ])
            // .stdin(Stdio::null())
            // .stdout(Stdio::piped())
            // .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| PgEmbedError::PgStartFailure(e))?;

        let exit_status = process
            .with_output_timeout(self.pg_settings.timeout)
            .strict_errors()
            .terminating()
            .wait()
            .map_err(|e| PgEmbedError::PgStartFailure(e))?
            .ok_or_else(|| PgEmbedError::PgStartFailure(Error::new(ErrorKind::TimedOut, "Postgresql startup command timed out")))?;

        if exit_status.status.success() {
            // println!("##### start_db success #####\n{}", String::from_utf8(exit_status.stdout).unwrap());
            self.server_status = PgServerStatus::Started;
            Ok(())
        } else {
            // println!("##### start_db error #####\n{}", String::from_utf8(exit_status.stderr).unwrap());
            self.server_status = PgServerStatus::Failure;
            Err(PgEmbedError::PgStartFailure(Error::new(ErrorKind::Other, format!("Postgresql startup command failed with {}", exit_status.status))))
        }
    }

    ///
    /// Stop postgresql database
    ///
    /// Returns `Ok(())` on success, otherwise returns an error.
    ///
    pub fn stop_db(&mut self) -> Result<(), PgEmbedError> {
        self.server_status = PgServerStatus::Stopping;
        let pg_ctl_executable = self.pg_access.pg_ctl_exe.as_str().unwrap();
        let mut process = Command::new(
            pg_ctl_executable,
        )
            .args(&[
                "stop", "-w", "-D", &self.pg_settings.database_dir.to_str().unwrap(),
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| PgEmbedError::PgStopFailure(e))?;


        let exit_status = process
            .with_output_timeout(self.pg_settings.timeout)
            .strict_errors()
            .terminating()
            .wait()
            .map_err(|e| PgEmbedError::PgStopFailure(e))?
            .ok_or_else(|| PgEmbedError::PgStopFailure(Error::new(ErrorKind::TimedOut, "Postgresql stop command timed out")))?;

        if exit_status.status.success() {
            info!(String::from_utf8(exit_status.stdout).unwrap());
            self.server_status = PgServerStatus::Stopped;
            Ok(())
        } else {
            error!(String::from_utf8(exit_status.stderr).unwrap());
            self.server_status = PgServerStatus::Failure;
            Err(PgEmbedError::PgStopFailure(Error::new(ErrorKind::Other, format!("Postgresql stop command failed with {}", exit_status.status))))
        }
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
            let m = Migrator::new(migration_dir.as_path()).await?;
            let pool = sqlx_tokio::postgres::PgPoolOptions::new().connect(&self.full_db_uri(db_name)).await?;
            m.run(&pool).await?;
        }
        Ok(())
    }
}