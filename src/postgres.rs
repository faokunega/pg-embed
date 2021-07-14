//!
//! Postgresql server
//!
//! Start, stop, initialize the postgresql server.
//! Create database clusters and databases.
//!
use futures::{TryFutureExt, StreamExt};
use std::process::{Command, Stdio, ExitStatus};
use crate::{pg_fetch, pg_unpack};
use crate::pg_errors::PgEmbedError;
#[cfg(any(feature = "rt_tokio", feature = "rt_tokio_migrate"))]
use tokio::io::AsyncWriteExt;
#[cfg(feature = "rt_tokio_migrate")]
use sqlx_tokio::Postgres;
#[cfg(feature = "rt_tokio_migrate")]
use sqlx_tokio::postgres::PgPoolOptions;
use std::time::Duration;
#[cfg(feature = "rt_tokio_migrate")]
use sqlx_tokio::migrate::{Migrator, MigrateDatabase};
use tokio::task;
use tokio::time::timeout;
use std::path::PathBuf;
use std::{io, thread};
use io::{Error, ErrorKind};
use log::{info, error};
use crate::pg_access::PgAccess;
use tokio::time::error::Elapsed;
use tokio::io::{BufReader, AsyncBufReadExt};
use tokio::process::Child;
use crate::pg_enums::{PgAuthMethod, PgServerStatus, PgProcessType, PgAcquisitionStatus};
use tokio::sync::Mutex;
use std::sync::Arc;
use std::collections::HashMap;


///
/// Database settings
///
pub struct PgSettings {
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
            tokio::runtime::Runtime::new()
                .expect("tokio runtime could not be created")
                .block_on(self.stop_db());
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
        if self.pg_access.acquisition_needed().await? {
            self.acquire_postgres().await?;
        }
        self.pg_access.create_password_file(self.pg_settings.password.as_bytes()).await?;
        if !self.pg_access.database_dir_exists().await? {
            &self.init_db().await?;
        }
        Ok(())
    }

    ///
    /// Download and unpack postgres binaries
    ///
    pub async fn acquire_postgres(&self) -> Result<(), PgEmbedError> {
        self.pg_access.mark_acquisition_in_progress().await?;
        let pg_bin_data = &self.fetch_settings.fetch_postgres().await?;
        self.pg_access.write_pg_zip(&pg_bin_data).await?;
        pg_unpack::unpack_postgres(&self.pg_access.zip_file_path, &self.pg_access.cache_dir).await?;
        self.pg_access.mark_acquisition_finished().await?;
        Ok(())
    }

    // pub async fn database_dir_status(&self) -> Result<>

    ///
    /// Initialize postgresql database
    ///
    /// Returns `Ok(())` on success, otherwise returns an error.
    ///
    pub async fn init_db(&mut self) -> Result<(), PgEmbedError> {
        self.server_status = PgServerStatus::Initializing;
        let mut init_db_command = self.pg_access.init_db_command(&self.pg_settings.database_dir, &self.pg_settings.user, &self.pg_settings.auth_method);
        let mut process = init_db_command.get_mut()
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| PgEmbedError::PgInitFailure(e))?;

        self.handle_process_io(&mut process).await?;

        self.timeout_pg_process(&mut process, PgProcessType::InitDb).await
    }

    ///
    /// Start postgresql database
    ///
    /// Returns `Ok(())` on success, otherwise returns an error.
    ///
    pub async fn start_db(&mut self) -> Result<(), PgEmbedError> {
        self.server_status = PgServerStatus::Starting;
        let mut start_db_command = self.pg_access.start_db_command(&self.pg_settings.database_dir, self.pg_settings.port);

        let mut process = start_db_command
            .get_mut()
            // .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| PgEmbedError::PgStartFailure(e))?;

        self.handle_process_io(&mut process).await?;

        self.timeout_pg_process(&mut process, PgProcessType::StartDb).await
    }

    ///
    /// Stop postgresql database
    ///
    /// Returns `Ok(())` on success, otherwise returns an error.
    ///
    pub async fn stop_db(&mut self) -> Result<(), PgEmbedError> {
        self.server_status = PgServerStatus::Stopping;
        let mut stop_db_command = self.pg_access.stop_db_command(&self.pg_settings.database_dir);
        let mut process = stop_db_command.get_mut()
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| PgEmbedError::PgStopFailure(e))?;

        self.handle_process_io(&mut process).await?;

        self.timeout_pg_process(&mut process, PgProcessType::StopDb).await
    }

    ///
    /// Execute postgresql process with timeout
    ///
    async fn timeout_pg_process(&mut self, process: &mut Child, process_type: PgProcessType) -> Result<(), PgEmbedError> {
        let timed_exit_status: Result<io::Result<ExitStatus>, Elapsed> = timeout(self.pg_settings.timeout, process.wait()).await;
        match timed_exit_status {
            Ok(exit_result) => {
                match exit_result {
                    Ok(exit_status) => {
                        if exit_status.success() {
                            match process_type {
                                PgProcessType::InitDb => {
                                    self.server_status = PgServerStatus::Initialized;
                                }
                                PgProcessType::StartDb => {
                                    self.server_status = PgServerStatus::Started;
                                }
                                PgProcessType::StopDb => {
                                    self.server_status = PgServerStatus::Stopped;
                                }
                            }
                            Ok(())
                        } else {
                            self.server_status = PgServerStatus::Failure;
                            Err(PgEmbedError::PgStartFailure(Error::new(ErrorKind::Other, format!("Postgresql {} command failed with {}", process_type.to_string(), exit_status))))
                        }
                    }
                    Err(err) => {
                        self.server_status = PgServerStatus::Failure;
                        Err(PgEmbedError::PgStartFailure(Error::new(ErrorKind::Other, format!("Postgresql {} command failed with {}", process_type.to_string(), err.to_string()))))
                    }
                }
            }
            Err(_) => {
                self.server_status = PgServerStatus::Failure;
                Err(PgEmbedError::PgStopFailure(Error::new(ErrorKind::TimedOut, format!("Postgresql {} command timed out", process_type.to_string()))))
            }
        }
    }

    ///
    /// Handle process logging
    ///
    pub async fn handle_process_io(&self, process: &mut Child) -> Result<(), PgEmbedError> {
        let mut reader_out = BufReader::new(process.stdout.take().unwrap()).lines();
        let mut reader_err = BufReader::new(process.stderr.take().unwrap()).lines();
        if let Some(line) = reader_out.next_line().await.map_err(|e| PgEmbedError::PgBufferReadError(e))? {
            info!("{}", line);
        }

        if let Some(line) = reader_err.next_line().await.map_err(|e| PgEmbedError::PgBufferReadError(e))? {
            error!("{}", line);
        }

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
            let m = Migrator::new(migration_dir.as_path()).await?;
            let pool = PgPoolOptions::new().connect(&self.full_db_uri(db_name)).await?;
            m.run(&pool).await?;
        }
        Ok(())
    }
}