//!
//! Cache postgresql files, access to executables, clean up files
//!

use std::path::PathBuf;

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
use futures::TryFutureExt;
use tokio::io::AsyncWriteExt;
use crate::postgres::PgSettings;
use crate::pg_fetch::{PgFetchSettings, OperationSystem};
use std::io::{Error, ErrorKind};
use crate::errors::errors_common::PgEmbedError::PgPurgeFailure;

///
/// Access to pg_ctl, initdb, database directory and cache directory
///
pub struct PgAccess {
    /// Cache directory path
    pub cache_dir: PathBuf,
    /// Database directory path
    pub database_dir: PathBuf,
    /// Postgresql pg_ctl executable path
    pub pg_ctl_exe: PathBuf,
    /// Postgresql initdb executable path
    pub init_db_exe: PathBuf,
    /// Password file path
    pub pw_file_path: PathBuf,
    /// Postgresql binaries zip file path
    pub zip_file_path: PathBuf,
}

impl PgAccess {
    ///
    /// Create a new instance
    ///
    /// Directory structure for cached postgresql binaries will be created
    ///
    pub async fn new(fetch_settings: &PgFetchSettings, database_dir: &PathBuf) -> Result<Self, PgEmbedError> {
        let cache_dir = Self::create_cache_dir_structure(&fetch_settings).await?;
        let mut pg_ctl = cache_dir.clone();
        pg_ctl.push("bin/pg_ctl");
        let mut init_db = cache_dir.clone();
        init_db.push("bin/initdb");
        let mut pw_file = cache_dir.clone();
        pw_file.push("pwfile");
        let mut zip_file_path = cache_dir.clone();
        let platform =
            fetch_settings.platform();
        let file_name = format!(
            "{}-{}.zip",
            platform,
            &settings.version.0
        );
        zip_file_path.push(file_name);

        Ok(
            PgAccess {
                cache_dir,
                database_dir: database_dir.clone(),
                pg_ctl_exe: pg_ctl,
                init_db_exe: init_db,
                pw_file_path: pw_file,
                zip_file_path,
            }
        )
    }

    ///
    /// Create directory structure for cached postgresql executables
    ///
    /// Returns PathBuf(cache_directory) on success, an error otherwise
    ///
    async fn create_cache_dir_structure(fetch_settings: &PgFetchSettings) -> Result<PathBuf, PgEmbedError> {
        let cache_dir =
            dirs::cache_dir().ok_or_else(
                || PgEmbedError::DirCreationError(Error::new(ErrorKind::Other, "cache dir error"))
            )?;
        let os_string = match fetch_settings.operating_system {
            OperationSystem::Darwin | OperationSystem::Windows | OperationSystem::Linux => fetch_settings.operating_system.to_string(),
            OperationSystem::AlpineLinux => format!("arch_{}", fetch_settings.operating_system.to_string())
        };
        let pg_path = format!("pg-embed/{}/{}/{}", os_string, fetch_settings.architecture.to_string(), fetch_settings.version.0);
        let mut cache_pg_embed = cache_dir.clone();
        cache_pg_embed.push(pg_path);
        tokio::fs::create_dir_all(
            &cache_pg_embed,
        ).map_err(|e| PgEmbedError::DirCreationError(e))
            .await?;
        Ok(cache_pg_embed)
    }

    ///
    /// Write to postgresql cache directory
    ///
    async fn write(&self, bytes: &[u8]) -> Result<(), PgEmbedError> {
        let mut file: tokio::fs::File =
            tokio::fs::File::create(&self.zip_file_path.as_path()).map_err(|e| PgEmbedError::WriteFileError(e)).await?;
        file.write_all(&bytes).map_err(|e| PgEmbedError::WriteFileError(e))
            .await?
    }

    ///
    /// Clean up created files and directories.
    ///
    /// Remove created directories containing the database and the password file.
    ///
    pub fn clean(&self) -> Result<(), PgEmbedError> {
        // not using tokio::fs async methods because clean() is called on drop
        std::fs::remove_dir_all(self.database_dir.as_path()).map_err(|e| PgEmbedError::PgCleanUpFailure(e))?;
        std::fs::remove_file(self.pw_file_path.as_path()).map_err(|e| PgEmbedError::PgCleanUpFailure(e))?;
        Ok(())
    }

    ///
    /// Purge postgresql executables
    ///
    /// Remove cached postgresql executables
    ///
    async fn purge(&self) -> Result<(), PgEmbedError> {
        tokio::fs::remove_dir_all(self.cache_dir.as_path()).map_err(|e| PgPurgeFailure(e)).await?
    }

    ///
    /// Create a database password file
    ///
    /// Returns `Ok(())` on success, otherwise returns an error.
    ///
    pub async fn create_password_file(&self, password: &[u8]) -> Result<(), PgEmbedError> {
        let mut file: tokio::fs::File = tokio::fs::File::create(self.zip_file_path.as_path()).map_err(|e| PgEmbedError::WriteFileError(e)).await?;
        let _ = file
            .write(password).map_err(|e| PgEmbedError::WriteFileError(e))
            .await?;
        Ok(())
    }
}