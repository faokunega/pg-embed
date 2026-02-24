//! Public API for embedding and managing a PostgreSQL server.
//!
//! The entry point is [`PgEmbed`].  A typical usage sequence is:
//!
//! ```rust,no_run
//! use pg_embed::postgres::{PgEmbed, PgSettings};
//! use pg_embed::pg_fetch::{PgFetchSettings, PG_V17};
//! use pg_embed::pg_enums::PgAuthMethod;
//! use std::path::PathBuf;
//! use std::time::Duration;
//!
//! # #[tokio::main]
//! # async fn main() -> pg_embed::pg_errors::Result<()> {
//! let pg_settings = PgSettings {
//!     database_dir: PathBuf::from("data/db"),
//!     port: 5432,
//!     user: "postgres".to_string(),
//!     password: "password".to_string(),
//!     auth_method: PgAuthMethod::Plain,
//!     persistent: false,
//!     timeout: Some(Duration::from_secs(15)),
//!     migration_dir: None,
//! };
//!
//! let fetch_settings = PgFetchSettings { version: PG_V17, ..Default::default() };
//!
//! let mut pg = PgEmbed::new(pg_settings, fetch_settings).await?;
//! pg.setup().await?;
//! pg.start_db().await?;
//!
//! let uri = pg.full_db_uri("mydb");   // postgres://postgres:password@localhost:5432/mydb
//!
//! pg.stop_db().await?;
//! # Ok(())
//! # }
//! ```

use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use log::{error, info};
use tokio::sync::Mutex;

#[cfg(feature = "rt_tokio_migrate")]
use sqlx::migrate::{MigrateDatabase, Migrator};
#[cfg(feature = "rt_tokio_migrate")]
use sqlx::postgres::PgPoolOptions;
#[cfg(feature = "rt_tokio_migrate")]
use sqlx::Postgres;

use crate::command_executor::AsyncCommand;
use crate::pg_access::PgAccess;
use crate::pg_commands::PgCommand;
use crate::pg_enums::{PgAuthMethod, PgServerStatus};
use crate::pg_errors::Error;
use crate::pg_errors::Result;
use crate::pg_fetch;

/// Configuration for a single embedded PostgreSQL instance.
pub struct PgSettings {
    /// Directory that will hold the PostgreSQL cluster data files.
    ///
    /// Created automatically if it does not exist.  When [`Self::persistent`]
    /// is `false` this directory (and [`Self::database_dir`] with a `.pwfile`
    /// extension) is removed when [`PgEmbed`] is dropped.
    pub database_dir: PathBuf,

    /// TCP port PostgreSQL will listen on.
    pub port: u16,

    /// Name of the initial database superuser.
    pub user: String,

    /// Password for the superuser, written to a temporary password file and
    /// passed to `initdb` via `--pwfile`.
    pub password: String,

    /// Authentication method written to `pg_hba.conf` by `initdb`.
    pub auth_method: PgAuthMethod,

    /// If `false`, the cluster directory and password file are deleted when
    /// the [`PgEmbed`] instance is dropped.  Set to `true` to keep the data
    /// across runs.
    pub persistent: bool,

    /// Maximum time to wait for `initdb`, `pg_ctl start`, and `pg_ctl stop`
    /// to complete.
    ///
    /// `None` disables the timeout (the process is waited on indefinitely).
    /// Exceeding the timeout returns [`Error::PgTimedOutError`].
    pub timeout: Option<Duration>,

    /// Directory containing `.sql` migration files.
    ///
    /// When `Some`, [`PgEmbed::migrate`] will run all migrations found in
    /// this directory via sqlx.  `None` disables migrations.
    /// Requires the `rt_tokio_migrate` feature.
    pub migration_dir: Option<PathBuf>,
}

/// An embedded PostgreSQL server with full lifecycle management.
///
/// Dropping a [`PgEmbed`] instance that has not been explicitly stopped will
/// automatically call `pg_ctl stop` synchronously and, if
/// [`PgSettings::persistent`] is `false`, remove the cluster directory and
/// password file.
pub struct PgEmbed {
    /// Active configuration for this instance.
    pub pg_settings: PgSettings,
    /// Binary download settings used during [`Self::setup`].
    pub fetch_settings: pg_fetch::PgFetchSettings,
    /// Base connection URI: `postgres://{user}:{password}@localhost:{port}`.
    pub db_uri: String,
    /// Current server lifecycle state, protected by an async mutex so it can
    /// be observed from concurrent tasks.
    pub server_status: Arc<Mutex<PgServerStatus>>,
    /// Set to `true` once a graceful stop has been initiated to prevent the
    /// `Drop` impl from issuing a duplicate stop.
    pub shutting_down: bool,
    /// File-system paths and I/O helpers for this instance.
    pub pg_access: PgAccess,
}

impl Drop for PgEmbed {
    fn drop(&mut self) {
        if !self.shutting_down {
            if let Err(e) = self.stop_db_sync() {
                log::warn!("pg_ctl stop failed during drop: {e}");
            }
        }
        if !self.pg_settings.persistent {
            if let Err(e) = self.pg_access.clean() {
                log::warn!("cleanup failed during drop: {e}");
            }
        }
    }
}

impl PgEmbed {
    /// Creates a new [`PgEmbed`] instance and prepares the directory structure.
    ///
    /// Does **not** download binaries or start the server.  Call
    /// [`Self::setup`] followed by [`Self::start_db`] to bring the server up.
    ///
    /// # Arguments
    ///
    /// * `pg_settings` — Server configuration (port, auth, directories, …).
    /// * `fetch_settings` — Which PostgreSQL version/platform to download.
    ///
    /// # Errors
    ///
    /// Returns [`Error::DirCreationError`] if the cache or database directories
    /// cannot be created.
    /// Returns [`Error::InvalidPgUrl`] if the OS cache directory is unavailable.
    pub async fn new(
        pg_settings: PgSettings,
        fetch_settings: pg_fetch::PgFetchSettings,
    ) -> Result<Self> {
        let db_uri = format!(
            "postgres://{}:{}@localhost:{}",
            &pg_settings.user, &pg_settings.password, pg_settings.port
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

    /// Downloads the binaries (if needed), writes the password file, and runs
    /// `initdb` (if the cluster does not already exist).
    ///
    /// This method is idempotent: if the binaries are already cached and the
    /// cluster is already initialised it returns immediately after verifying
    /// both.
    ///
    /// # Errors
    ///
    /// Returns any error from [`PgAccess::maybe_acquire_postgres`],
    /// [`PgAccess::create_password_file`], or [`Self::init_db`].
    pub async fn setup(&mut self) -> Result<()> {
        self.pg_access.maybe_acquire_postgres().await?;
        self.pg_access
            .create_password_file(self.pg_settings.password.as_bytes())
            .await?;
        if self.pg_access.db_files_exist().await? {
            let mut server_status = self.server_status.lock().await;
            *server_status = PgServerStatus::Initialized;
        } else {
            self.init_db().await?;
        }
        Ok(())
    }

    /// Installs a third-party PostgreSQL extension into the binary cache.
    ///
    /// Must be called **after** [`Self::setup`] (so the cache directory exists)
    /// and **before** [`Self::start_db`] (so the server loads the shared
    /// library on startup).  Once the server is running, activate the extension
    /// in a specific database with:
    ///
    /// ```sql
    /// CREATE EXTENSION IF NOT EXISTS <extension_name>;
    /// ```
    ///
    /// Delegates to [`PgAccess::install_extension`].  See that method for the
    /// file-routing rules (`.so`/`.dylib`/`.dll` → `lib/`;
    /// `.control`/`.sql` → the PostgreSQL share extension directory).
    ///
    /// # Arguments
    ///
    /// * `extension_dir` — Directory containing the pre-compiled extension
    ///   files (shared library + control + SQL scripts).
    ///
    /// # Errors
    ///
    /// Returns [`Error::DirCreationError`] if the target directories cannot be
    /// created.
    /// Returns [`Error::ReadFileError`] if `extension_dir` cannot be read.
    /// Returns [`Error::WriteFileError`] if a file cannot be copied.
    pub async fn install_extension(&self, extension_dir: &Path) -> Result<()> {
        self.pg_access.install_extension(extension_dir).await
    }

    /// Runs `initdb` to create a new database cluster.
    ///
    /// Updates [`Self::server_status`] to [`PgServerStatus::Initializing`]
    /// before the call and to [`PgServerStatus::Initialized`] on success.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidPgUrl`] if any path cannot be converted to UTF-8.
    /// Returns [`Error::PgInitFailure`] if `initdb` cannot be spawned.
    /// Returns [`Error::PgTimedOutError`] if the process exceeds
    /// [`PgSettings::timeout`].
    pub async fn init_db(&mut self) -> Result<()> {
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

    /// Starts the PostgreSQL server with `pg_ctl start -w`.
    ///
    /// Updates [`Self::server_status`] to [`PgServerStatus::Starting`] before
    /// the call and to [`PgServerStatus::Started`] on success.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidPgUrl`] if the cluster path cannot be converted
    /// to UTF-8.
    /// Returns [`Error::PgStartFailure`] if the process exits with a non-zero
    /// status or cannot be spawned.
    /// Returns [`Error::PgTimedOutError`] if the process exceeds
    /// [`PgSettings::timeout`].
    pub async fn start_db(&mut self) -> Result<()> {
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

    /// Stops the PostgreSQL server with `pg_ctl stop -w`.
    ///
    /// Updates [`Self::server_status`] to [`PgServerStatus::Stopping`] before
    /// the call and to [`PgServerStatus::Stopped`] on success.  Sets
    /// [`Self::shutting_down`] to `true` so the `Drop` impl does not issue a
    /// duplicate stop.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidPgUrl`] if the cluster path cannot be converted
    /// to UTF-8.
    /// Returns [`Error::PgStopFailure`] if `pg_ctl stop` fails.
    /// Returns [`Error::PgTimedOutError`] if the process exceeds
    /// [`PgSettings::timeout`].
    pub async fn stop_db(&mut self) -> Result<()> {
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

    /// Stops the PostgreSQL server synchronously.
    ///
    /// Used by the `Drop` impl where async is unavailable.  Stdout and stderr
    /// of the `pg_ctl stop` process are forwarded to the [`log`] crate.
    ///
    /// # Errors
    ///
    /// Returns [`Error::PgError`] if the process cannot be spawned.
    pub fn stop_db_sync(&mut self) -> Result<()> {
        self.shutting_down = true;
        let mut stop_db_command = self
            .pg_access
            .stop_db_command_sync(&self.pg_settings.database_dir);
        let process = stop_db_command
            .get_mut()
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| Error::PgError(e.to_string(), "".to_string()))?;

        self.handle_process_io_sync(process)
    }

    /// Drains stdout and stderr of `process`, logging each line.
    ///
    /// Lines from stdout are logged at `info` level; lines from stderr at
    /// `error` level.  Read errors are silently ignored (the line is skipped).
    ///
    /// # Arguments
    ///
    /// * `process` — A child process with piped stdout/stderr.
    pub fn handle_process_io_sync(&self, mut process: std::process::Child) -> Result<()> {
        if let Some(stdout) = process.stdout.take() {
            std::io::BufReader::new(stdout)
                .lines()
                .for_each(|line| {
                    if let Ok(l) = line {
                        info!("{}", l);
                    }
                });
        }
        if let Some(stderr) = process.stderr.take() {
            std::io::BufReader::new(stderr)
                .lines()
                .for_each(|line| {
                    if let Ok(l) = line {
                        error!("{}", l);
                    }
                });
        }
        Ok(())
    }

    /// Creates a new PostgreSQL database.
    ///
    /// Requires the `rt_tokio_migrate` feature.
    ///
    /// # Arguments
    ///
    /// * `db_name` — Name of the database to create.
    ///
    /// # Errors
    ///
    /// Returns [`Error::PgTaskJoinError`] if the sqlx operation fails.
    #[cfg(feature = "rt_tokio_migrate")]
    pub async fn create_database(&self, db_name: &str) -> Result<()> {
        Postgres::create_database(&self.full_db_uri(db_name))
            .await
            .map_err(|e| Error::PgTaskJoinError(e.to_string()))?;
        Ok(())
    }

    /// Drops a PostgreSQL database if it exists.
    ///
    /// Uses `DROP DATABASE IF EXISTS` semantics: if the database does not
    /// exist the call succeeds silently.
    /// Requires the `rt_tokio_migrate` feature.
    ///
    /// # Arguments
    ///
    /// * `db_name` — Name of the database to drop.
    ///
    /// # Errors
    ///
    /// Returns [`Error::PgTaskJoinError`] if the sqlx operation fails.
    #[cfg(feature = "rt_tokio_migrate")]
    pub async fn drop_database(&self, db_name: &str) -> Result<()> {
        Postgres::drop_database(&self.full_db_uri(db_name))
            .await
            .map_err(|e| Error::PgTaskJoinError(e.to_string()))?;
        Ok(())
    }

    /// Returns `true` if a database named `db_name` exists.
    ///
    /// Requires the `rt_tokio_migrate` feature.
    ///
    /// # Arguments
    ///
    /// * `db_name` — Name of the database to check.
    ///
    /// # Errors
    ///
    /// Returns [`Error::PgTaskJoinError`] if the sqlx operation fails.
    #[cfg(feature = "rt_tokio_migrate")]
    pub async fn database_exists(&self, db_name: &str) -> Result<bool> {
        Postgres::database_exists(&self.full_db_uri(db_name))
            .await
            .map_err(|e| Error::PgTaskJoinError(e.to_string()))
    }

    /// Returns the full connection URI for a specific database.
    ///
    /// Format: `postgres://{user}:{password}@localhost:{port}/{db_name}`.
    ///
    /// # Arguments
    ///
    /// * `db_name` — Database name to append to the base URI.
    pub fn full_db_uri(&self, db_name: &str) -> String {
        format!("{}/{}", &self.db_uri, db_name)
    }

    /// Runs sqlx migrations from [`PgSettings::migration_dir`] against `db_name`.
    ///
    /// Does nothing if [`PgSettings::migration_dir`] is `None`.
    /// Requires the `rt_tokio_migrate` feature.
    ///
    /// # Arguments
    ///
    /// * `db_name` — Name of the target database.
    ///
    /// # Errors
    ///
    /// Returns [`Error::MigrationError`] if the migrator cannot be created or
    /// if a migration fails.
    /// Returns [`Error::SqlQueryError`] if the database connection fails.
    #[cfg(feature = "rt_tokio_migrate")]
    pub async fn migrate(&self, db_name: &str) -> Result<()> {
        if let Some(migration_dir) = &self.pg_settings.migration_dir {
            let m = Migrator::new(migration_dir.as_path())
                .await
                .map_err(|e| Error::MigrationError(e.to_string()))?;
            let pool = PgPoolOptions::new()
                .connect(&self.full_db_uri(db_name))
                .await
                .map_err(|e| Error::SqlQueryError(e.to_string()))?;
            m.run(&pool)
                .await
                .map_err(|e| Error::MigrationError(e.to_string()))?;
        }
        Ok(())
    }
}
