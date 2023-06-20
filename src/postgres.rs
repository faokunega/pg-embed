//!
//! Postgresql server
//!
//! Start, stop, initialize the postgresql server.
//! Create database clusters and databases.
//!
use std::io::BufRead;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use futures::TryFutureExt;
use log::{error, info};
use tokio::sync::Mutex;

#[cfg(feature = "rt_tokio_migrate")]
use sqlx_tokio::migrate::{MigrateDatabase, Migrator};
#[cfg(feature = "rt_tokio_migrate")]
use sqlx_tokio::postgres::PgPoolOptions;
#[cfg(feature = "rt_tokio_migrate")]
use sqlx_tokio::Postgres;

use crate::command_executor::AsyncCommand;
use crate::pg_access::PgAccess;
use crate::pg_commands::PgCommand;
use crate::pg_enums::{PgAuthMethod, PgServerStatus};
use crate::pg_errors::{PgEmbedError, PgEmbedErrorType};
use crate::pg_fetch;
use crate::pg_types::PgResult;

///
/// Database settings
///
pub struct PgSettings {
    /// postgresql database directory
    pub database_dir: PathBuf,
    /// postgresql port
    pub port: u16,
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
    pub timeout: Option<Duration>,
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
    pub server_status: Arc<Mutex<PgServerStatus>>,
    pub shutting_down: bool,
    /// Postgres files access
    pub pg_access: PgAccess,
}

impl Drop for PgEmbed {
    fn drop(&mut self) {
        if !self.shutting_down {
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
    pub async fn new(
        pg_settings: PgSettings,
        fetch_settings: pg_fetch::PgFetchSettings,
    ) -> PgResult<Self> {
        let password: &str = &pg_settings.password;
        let db_uri = format!(
            "postgres://{}:{}@localhost:{}",
            &pg_settings.user,
            &password,
            pg_settings.port.to_string()
        );
        let pg_access = PgAccess::new(&fetch_settings, &pg_settings.database_dir).await?;
        Ok(PgEmbed {
            pg_settings,
            fetch_settings,
            db_uri,
            server_status: Arc::new(Mutex::new(PgServerStatus::Uninitialized)),
            shutting_down: false,
            pg_access,
        })
    }

    ///
    /// Setup postgresql for execution
    ///
    /// Download, unpack, create password file and database
    ///
    pub async fn setup(&mut self) -> PgResult<()> {
        self.pg_access.maybe_acquire_postgres().await?;
        self.pg_access
            .create_password_file(self.pg_settings.password.as_bytes())
            .await?;
        if self.pg_access.db_files_exist().await? {
            let mut server_status = self.server_status.lock().await;
            *server_status = PgServerStatus::Initialized;
        } else {
            let _r = &self.init_db().await?;
        }
        Ok(())
    }

    ///
    /// Initialize postgresql database
    ///
    /// Returns `Ok(())` on success, otherwise returns an error.
    ///
    pub async fn init_db(&mut self) -> PgResult<()> {
        {
            let mut server_status = self.server_status.lock().await;
            *server_status = PgServerStatus::Initializing;
        }

        let mut executor = PgCommand::init_db_executor(
            &self.pg_access.init_db_exe,
            &self.pg_access.database_dir,
            &self.pg_access.pw_file_path,
            &self.pg_settings.user,
            &self.pg_settings.auth_method,
        )?;
        let exit_status = executor.execute(self.pg_settings.timeout).await?;
        let mut server_status = self.server_status.lock().await;
        *server_status = exit_status;
        Ok(())
    }

    ///
    /// Start postgresql database
    ///
    /// Returns `Ok(())` on success, otherwise returns an error.
    ///
    pub async fn start_db(&mut self) -> PgResult<()> {
        {
            let mut server_status = self.server_status.lock().await;
            *server_status = PgServerStatus::Starting;
        }
        self.shutting_down = false;
        let mut executor = PgCommand::start_db_executor(
            &self.pg_access.pg_ctl_exe,
            &self.pg_access.database_dir,
            &self.pg_settings.port,
        )?;
        let exit_status = executor.execute(self.pg_settings.timeout).await?;
        let mut server_status = self.server_status.lock().await;
        *server_status = exit_status;
        Ok(())
    }

    ///
    /// Stop postgresql database
    ///
    /// Returns `Ok(())` on success, otherwise returns an error.
    ///
    pub async fn stop_db(&mut self) -> PgResult<()> {
        {
            let mut server_status = self.server_status.lock().await;
            *server_status = PgServerStatus::Stopping;
        }
        self.shutting_down = true;
        let mut executor =
            PgCommand::stop_db_executor(&self.pg_access.pg_ctl_exe, &self.pg_access.database_dir)?;
        let exit_status = executor.execute(self.pg_settings.timeout).await?;
        let mut server_status = self.server_status.lock().await;
        *server_status = exit_status;
        Ok(())
    }

    ///
    /// Stop postgresql database synchronous
    ///
    /// Returns `Ok(())` on success, otherwise returns an error.
    ///
    pub fn stop_db_sync(&mut self) -> PgResult<()> {
        self.shutting_down = true;
        let mut stop_db_command = self
            .pg_access
            .stop_db_command_sync(&self.pg_settings.database_dir);
        let process = stop_db_command
            .get_mut()
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| PgEmbedError {
                error_type: PgEmbedErrorType::PgError,
                source: Some(Box::new(e)),
                message: None,
            })?;

        self.handle_process_io_sync(process)
    }

    ///
    /// Handle process logging synchronous
    ///
    pub fn handle_process_io_sync(&self, mut process: std::process::Child) -> PgResult<()> {
        let reader_out = std::io::BufReader::new(process.stdout.take().unwrap()).lines();
        let reader_err = std::io::BufReader::new(process.stderr.take().unwrap()).lines();
        reader_out.for_each(|line| info!("{}", line.unwrap()));
        reader_err.for_each(|line| error!("{}", line.unwrap()));
        Ok(())
    }

    ///
    /// Create a database
    ///
    #[cfg(any(
        feature = "rt_tokio_migrate",
        feature = "rt_async_std_migrate",
        feature = "rt_actix_migrate"
    ))]
    pub async fn create_database(&self, db_name: &str) -> PgResult<()> {
        Postgres::create_database(&self.full_db_uri(db_name))
            .map_err(|e| PgEmbedError {
                error_type: PgEmbedErrorType::PgTaskJoinError,
                source: Some(Box::new(e)),
                message: None,
            })
            .await?;
        Ok(())
    }

    ///
    /// Drop a database
    ///
    #[cfg(any(
        feature = "rt_tokio_migrate",
        feature = "rt_async_std_migrate",
        feature = "rt_actix_migrate"
    ))]
    pub async fn drop_database(&self, db_name: &str) -> PgResult<()> {
        Postgres::drop_database(&self.full_db_uri(db_name))
            .map_err(|e| PgEmbedError {
                error_type: PgEmbedErrorType::PgTaskJoinError,
                source: Some(Box::new(e)),
                message: None,
            })
            .await?;
        Ok(())
    }

    ///
    /// Check database existence
    ///
    #[cfg(any(
        feature = "rt_tokio_migrate",
        feature = "rt_async_std_migrate",
        feature = "rt_actix_migrate"
    ))]
    pub async fn database_exists(&self, db_name: &str) -> PgResult<bool> {
        let result = Postgres::database_exists(&self.full_db_uri(db_name))
            .map_err(|e| PgEmbedError {
                error_type: PgEmbedErrorType::PgTaskJoinError,
                source: Some(Box::new(e)),
                message: None,
            })
            .await?;
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
    #[cfg(any(
        feature = "rt_tokio_migrate",
        feature = "rt_async_std_migrate",
        feature = "rt_actix_migrate"
    ))]
    pub async fn migrate(&self, db_name: &str) -> PgResult<()> {
        if let Some(migration_dir) = &self.pg_settings.migration_dir {
            let m = Migrator::new(migration_dir.as_path())
                .map_err(|e| PgEmbedError {
                    error_type: PgEmbedErrorType::MigrationError,
                    source: Some(Box::new(e)),
                    message: None,
                })
                .await?;
            let pool = PgPoolOptions::new()
                .connect(&self.full_db_uri(db_name))
                .map_err(|e| PgEmbedError {
                    error_type: PgEmbedErrorType::SqlQueryError,
                    source: Some(Box::new(e)),
                    message: None,
                })
                .await?;
            m.run(&pool)
                .map_err(|e| PgEmbedError {
                    error_type: PgEmbedErrorType::MigrationError,
                    source: Some(Box::new(e)),
                    message: None,
                })
                .await?;
        }
        Ok(())
    }
}
