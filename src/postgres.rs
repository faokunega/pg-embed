//!
//! Postgresql server
//!
//! Start, stop, initialize the postgresql server.
//! Create database clusters and databases.
//!
use io::{Error, ErrorKind};
use std::io;
use std::io::BufRead;
use std::path::PathBuf;
use std::process::{ExitStatus, Stdio};
use std::time::Duration;

use log::{error, info};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Child;
use tokio::time::error::Elapsed;
use tokio::time::timeout;

#[cfg(feature = "rt_tokio_migrate")]
use sqlx_tokio::migrate::{MigrateDatabase, Migrator};
#[cfg(feature = "rt_tokio_migrate")]
use sqlx_tokio::Postgres;
#[cfg(feature = "rt_tokio_migrate")]
use sqlx_tokio::postgres::PgPoolOptions;

use crate::{pg_fetch, pg_unpack};
use crate::pg_access::PgAccess;
use crate::pg_enums::{PgAuthMethod, PgProcessType, PgServerStatus};
use crate::pg_errors::PgEmbedError;
use crate::pg_types::{PgResult, PgCommand};

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
        if self.server_status != PgServerStatus::Stopped
            || self.server_status != PgServerStatus::Stopping
        {
            let _ = self.stop_db_sync();
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
    pub async fn new(pg_settings: PgSettings, fetch_settings: pg_fetch::PgFetchSettings) -> PgResult<Self> {
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
    pub async fn setup(&mut self) -> PgResult<()> {
        if self.pg_access.acquisition_needed().await? {
            self.acquire_postgres().await?;
        }
        self.pg_access.create_password_file(self.pg_settings.password.as_bytes()).await?;
        if self.pg_access.db_files_exist().await? {
            self.server_status = PgServerStatus::Initialized;
        } else {
            &self.init_db().await?;
        }
        Ok(())
    }

    ///
    /// Download and unpack postgres binaries
    ///
    pub async fn acquire_postgres(&self) -> PgResult<()> {
        self.pg_access.mark_acquisition_in_progress().await?;
        let pg_bin_data = &self.fetch_settings.fetch_postgres().await?;
        self.pg_access.write_pg_zip(&pg_bin_data).await?;
        pg_unpack::unpack_postgres(&self.pg_access.zip_file_path, &self.pg_access.cache_dir).await?;
        self.pg_access.mark_acquisition_finished().await?;
        Ok(())
    }

    ///
    /// Get child process for a pg command
    ///
    /// The child process's stdout and stderr are piped
    ///
    fn pg_command_child_process(command: &mut PgCommand, process_type: PgProcessType) -> PgResult<Child> {
        command.get_mut()
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| match process_type {
                PgProcessType::InitDb => {
                    PgEmbedError::PgInitFailure(e)
                }
                PgProcessType::StartDb => {
                    PgEmbedError::PgStartFailure(e)
                }
                PgProcessType::StopDb => {
                    PgEmbedError::PgStopFailure(e)
                }
            })
    }

    ///
    /// Initialize postgresql database
    ///
    /// Returns `Ok(())` on success, otherwise returns an error.
    ///
    pub async fn init_db(&mut self) -> PgResult<()> {
        self.server_status = PgServerStatus::Initializing;
        let mut init_db_command = self.pg_access.init_db_command(&self.pg_settings.database_dir, &self.pg_settings.user, &self.pg_settings.auth_method);
        let mut process = Self::pg_command_child_process(&mut init_db_command, PgProcessType::InitDb)?;
        self.handle_process_io(&mut process).await?;
        self.timeout_pg_process(&mut process, PgProcessType::InitDb).await
    }

    ///
    /// Start postgresql database
    ///
    /// Returns `Ok(())` on success, otherwise returns an error.
    ///
    pub async fn start_db(&mut self) -> PgResult<()> {
        self.server_status = PgServerStatus::Starting;
        let mut start_db_command = self.pg_access.start_db_command(&self.pg_settings.database_dir, self.pg_settings.port);
        let mut process = Self::pg_command_child_process(&mut start_db_command, PgProcessType::StartDb)?;
        self.handle_process_io(&mut process).await?;
        self.timeout_pg_process(&mut process, PgProcessType::StartDb).await
    }

    ///
    /// Stop postgresql database
    ///
    /// Returns `Ok(())` on success, otherwise returns an error.
    ///
    pub async fn stop_db(&mut self) -> PgResult<()> {
        self.server_status = PgServerStatus::Stopping;
        let mut stop_db_command = self.pg_access.stop_db_command(&self.pg_settings.database_dir);
        let mut process = Self::pg_command_child_process(&mut stop_db_command, PgProcessType::StopDb)?;
        self.handle_process_io(&mut process).await?;
        self.timeout_pg_process(&mut process, PgProcessType::StopDb).await
    }


    ///
    /// Stop postgresql database synchronous
    ///
    /// Returns `Ok(())` on success, otherwise returns an error.
    ///
    pub fn stop_db_sync(&mut self) -> PgResult<()> {
        self.server_status = PgServerStatus::Stopping;

        let mut stop_db_command = self.pg_access.stop_db_command_sync(&self.pg_settings.database_dir);
        let mut process = stop_db_command.get_mut()
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| PgEmbedError::PgStopFailure(e))?;

        self.handle_process_io_sync(&mut process)
    }

    ///
    /// Get process error for corresponding process type
    ///
    fn process_type_failure(process_type: PgProcessType, exit_code: Option<i32>) -> PgEmbedError {
        match process_type {
            PgProcessType::InitDb => {
                PgEmbedError::PgInitFailure(
                    Error::new(
                        ErrorKind::Other,
                        format!("Postgresql {} command failure with exit status {:?}", process_type.to_string(), exit_code),
                    )
                )
            }
            PgProcessType::StartDb => {
                PgEmbedError::PgStartFailure(
                    Error::new(
                        ErrorKind::Other,
                        format!("Postgresql {} command failure with exit status {:?}", process_type.to_string(), exit_code),
                    )
                )
            }
            PgProcessType::StopDb => {
                PgEmbedError::PgStopFailure(
                    Error::new(
                        ErrorKind::Other,
                        format!("Postgresql {} command failure with exit status {:?}", process_type.to_string(), exit_code),
                    )
                )
            }
        }
    }

    ///
    /// Get server status for corresponding exit status
    ///
    fn process_type_server_status(exit_status: io::Result<ExitStatus>, process_type: PgProcessType) -> PgResult<PgServerStatus> {
        let mut exit_code: Option<i32> = None;
        if let Ok(status) = exit_status {
            exit_code = status.code();
            if status.success() {
                return match process_type {
                    PgProcessType::InitDb => Ok(PgServerStatus::Initialized),
                    PgProcessType::StartDb => Ok(PgServerStatus::Started),
                    PgProcessType::StopDb => Ok(PgServerStatus::Stopped),
                };
            }
        }
        Err(Self::process_type_failure(process_type, exit_code))
    }

    ///
    /// Execute postgresql process with timeout
    ///
    async fn timeout_pg_process(&mut self, process: &mut Child, process_type: PgProcessType) -> PgResult<()> {
        let timed_exit_status: Result<io::Result<ExitStatus>, Elapsed> = timeout(self.pg_settings.timeout, process.wait()).await;
        if let Ok(exit_result) = timed_exit_status {
            let status = Self::process_type_server_status(exit_result, process_type)?;
            self.server_status = status;
            Ok(())
        } else {
            self.server_status = PgServerStatus::Failure;
            Err(PgEmbedError::PgTimedOutError())
        }
    }

    ///
    /// Handle process logging
    ///
    pub async fn handle_process_io(&self, process: &mut Child) -> PgResult<()> {
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
    /// Handle process logging synchronous
    ///
    pub fn handle_process_io_sync(&self, process: &mut std::process::Child) -> PgResult<()> {
        let reader_out = std::io::BufReader::new(process.stdout.take().unwrap()).lines();
        let reader_err = std::io::BufReader::new(process.stderr.take().unwrap()).lines();
        reader_out.for_each(|line| info!("{}", line.unwrap()));
        reader_err.for_each(|line| error!("{}", line.unwrap()));
        Ok(())
    }

    ///
    /// Create a database
    ///
    #[cfg(any(feature = "rt_tokio_migrate", feature = "rt_async_std_migrate", feature = "rt_actix_migrate"))]
    pub async fn create_database(&self, db_name: &str) -> PgResult<()> {
        Postgres::create_database(&self.full_db_uri(db_name)).await?;
        Ok(())
    }

    ///
    /// Drop a database
    ///
    #[cfg(any(feature = "rt_tokio_migrate", feature = "rt_async_std_migrate", feature = "rt_actix_migrate"))]
    pub async fn drop_database(&self, db_name: &str) -> PgResult<()> {
        Postgres::drop_database(&self.full_db_uri(db_name)).await?;
        Ok(())
    }

    ///
    /// Check database existence
    ///
    #[cfg(any(feature = "rt_tokio_migrate", feature = "rt_async_std_migrate", feature = "rt_actix_migrate"))]
    pub async fn database_exists(&self, db_name: &str) -> PgResult<bool> {
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
    pub async fn migrate(&self, db_name: &str) -> PgResult<()> {
        if let Some(migration_dir) = &self.pg_settings.migration_dir {
            let m = Migrator::new(migration_dir.as_path()).await?;
            let pool = PgPoolOptions::new().connect(&self.full_db_uri(db_name)).await?;
            m.run(&pool).await?;
        }
        Ok(())
    }
}