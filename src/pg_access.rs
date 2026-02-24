//! File-system access layer for cached PostgreSQL binaries and database clusters.
//!
//! [`PgAccess`] encapsulates all paths used by pg-embed (cache dir, database
//! dir, executable paths, password file) and provides the operations that act
//! on those paths: downloading, unpacking, writing the password file, and
//! cleaning up.
//!
//! The module-level static `ACQUIRED_PG_BINS` prevents concurrent downloads
//! of the same binaries when multiple [`crate::postgres::PgEmbed`] instances
//! start simultaneously.

use std::cell::Cell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock};

use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

use crate::pg_enums::{OperationSystem, PgAcquisitionStatus};
use crate::pg_errors::Error;
use crate::pg_fetch::PgFetchSettings;
use crate::pg_types::PgCommandSync;
use crate::pg_unpack;
use crate::pg_errors::Result;

/// Guards concurrent binary downloads across multiple [`crate::postgres::PgEmbed`] instances.
///
/// The key is the cache directory path; the value tracks whether acquisition
/// is in progress or finished.  Protected by a [`Mutex`] to allow only one
/// download per unique cache path at a time.
static ACQUIRED_PG_BINS: LazyLock<Arc<Mutex<HashMap<PathBuf, PgAcquisitionStatus>>>> =
    LazyLock::new(|| Arc::new(Mutex::new(HashMap::with_capacity(5))));

const PG_EMBED_CACHE_DIR_NAME: &str = "pg-embed";
const PG_VERSION_FILE_NAME: &str = "PG_VERSION";

/// Manages all file-system paths and I/O operations for a single pg-embed instance.
///
/// Created by [`PgAccess::new`], which also creates the required directory
/// structure.  All path fields are derived from the fetch settings and the
/// caller-supplied database directory.
///
/// # Cache layout
///
/// ```text
/// {cache_dir}/pg-embed/{os}/{arch}/{version}/
///   bin/pg_ctl
///   bin/initdb
///   {platform}-{version}.zip
/// ```
pub struct PgAccess {
    /// Root of the per-version binary cache.
    pub cache_dir: PathBuf,
    /// Directory that holds the PostgreSQL cluster data files.
    pub database_dir: PathBuf,
    /// Path to the `pg_ctl` executable inside the cache.
    pub pg_ctl_exe: PathBuf,
    /// Path to the `initdb` executable inside the cache.
    pub init_db_exe: PathBuf,
    /// Path to the password file used by `initdb`.
    pub pw_file_path: PathBuf,
    /// Path where the downloaded JAR is written before unpacking.
    pub zip_file_path: PathBuf,
    /// `PG_VERSION` file inside the cluster directory; used to detect an
    /// already-initialised cluster.
    pg_version_file: PathBuf,
    /// Download settings used to reconstruct the cache path.
    fetch_settings: PgFetchSettings,
}

impl PgAccess {
    /// Creates a new [`PgAccess`] and ensures the required directories exist.
    ///
    /// Both the per-version binary cache directory and `database_dir` are
    /// created with [`tokio::fs::create_dir_all`] if they do not already exist.
    ///
    /// # Arguments
    ///
    /// * `fetch_settings` — Determines the OS, architecture, and version used
    ///   to construct the cache path.
    /// * `database_dir` — Where the PostgreSQL cluster data files will live.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidPgUrl`] if the OS cache directory cannot be
    /// resolved.
    /// Returns [`Error::DirCreationError`] if either directory cannot be
    /// created.
    pub async fn new(
        fetch_settings: &PgFetchSettings,
        database_dir: &Path,
    ) -> Result<Self> {
        let cache_dir = Self::create_cache_dir_structure(fetch_settings).await?;
        Self::create_db_dir_structure(database_dir).await?;
        let platform = fetch_settings.platform();
        let pg_ctl = cache_dir.join("bin/pg_ctl");
        let init_db = cache_dir.join("bin/initdb");
        let zip_file_path = cache_dir.join(format!("{}-{}.zip", platform, fetch_settings.version.0));
        let mut pw_file = database_dir.to_path_buf();
        pw_file.set_extension("pwfile");
        let pg_version_file = database_dir.join(PG_VERSION_FILE_NAME);

        Ok(PgAccess {
            cache_dir,
            database_dir: database_dir.to_path_buf(),
            pg_ctl_exe: pg_ctl,
            init_db_exe: init_db,
            pw_file_path: pw_file,
            zip_file_path,
            pg_version_file,
            fetch_settings: fetch_settings.clone(),
        })
    }

    /// Creates the OS-specific cache directory tree for this OS/arch/version.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidPgUrl`] if the OS cache directory cannot be
    /// resolved.
    /// Returns [`Error::DirCreationError`] if the directory cannot be created.
    async fn create_cache_dir_structure(fetch_settings: &PgFetchSettings) -> Result<PathBuf> {
        let cache_dir = dirs::cache_dir().ok_or(Error::InvalidPgUrl)?;
        let os_string = match fetch_settings.operating_system {
            OperationSystem::Darwin | OperationSystem::Windows | OperationSystem::Linux => {
                fetch_settings.operating_system.to_string()
            }
            OperationSystem::AlpineLinux => {
                format!("arch_{}", fetch_settings.operating_system)
            }
        };
        let pg_path = format!(
            "{}/{}/{}/{}",
            PG_EMBED_CACHE_DIR_NAME,
            os_string,
            fetch_settings.architecture,
            fetch_settings.version.0
        );
        let mut cache_pg_embed = cache_dir;
        cache_pg_embed.push(pg_path);
        tokio::fs::create_dir_all(&cache_pg_embed)
            .await
            .map_err(|e| Error::DirCreationError(e.to_string()))?;
        Ok(cache_pg_embed)
    }

    /// Creates the database cluster directory.
    ///
    /// # Errors
    ///
    /// Returns [`Error::DirCreationError`] if the directory cannot be created.
    async fn create_db_dir_structure(db_dir: &Path) -> Result<()> {
        tokio::fs::create_dir_all(db_dir)
            .await
            .map_err(|e| Error::DirCreationError(e.to_string()))
    }

    /// Downloads and unpacks the PostgreSQL binaries if they are not already cached.
    ///
    /// Acquires the `ACQUIRED_PG_BINS` lock for the duration.  If another
    /// instance already cached the binaries (i.e. [`Self::pg_executables_cached`]
    /// returns `true`), this method returns immediately without downloading.
    ///
    /// # Errors
    ///
    /// Returns [`Error::DirCreationError`] if directories cannot be created.
    /// Returns [`Error::DownloadFailure`] or [`Error::ConversionFailure`] if
    /// the HTTP download fails.
    /// Returns [`Error::WriteFileError`] if the JAR cannot be written to disk.
    /// Returns [`Error::UnpackFailure`] or [`Error::InvalidPgPackage`] if
    /// extraction fails.
    pub async fn maybe_acquire_postgres(&self) -> Result<()> {
        let mut lock = ACQUIRED_PG_BINS.lock().await;

        if self.pg_executables_cached().await? {
            return Ok(());
        }

        lock.insert(self.cache_dir.clone(), PgAcquisitionStatus::InProgress);
        self.fetch_settings
            .fetch_postgres_to_file(&self.zip_file_path)
            .await?;
        log::debug!(
            "Unpacking postgres binaries {} {}",
            self.zip_file_path.display(),
            self.cache_dir.display()
        );
        pg_unpack::unpack_postgres(&self.zip_file_path, &self.cache_dir).await?;

        if let Some(status) = lock.get_mut(&self.cache_dir) {
            *status = PgAcquisitionStatus::Finished;
        }
        Ok(())
    }

    /// Returns `true` if the `initdb` executable is present in the cache.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ReadFileError`] if the filesystem existence check fails.
    pub async fn pg_executables_cached(&self) -> Result<bool> {
        Self::path_exists(self.init_db_exe.as_path()).await
    }

    /// Returns `true` if both the executables and the cluster version file exist.
    ///
    /// A `true` result indicates the cluster was previously initialised with
    /// `initdb` and does not need to be re-initialised.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ReadFileError`] if either filesystem check fails.
    pub async fn db_files_exist(&self) -> Result<bool> {
        Ok(self.pg_executables_cached().await?
            && Self::path_exists(self.pg_version_file.as_path()).await?)
    }

    /// Returns `true` if the `PG_VERSION` file exists inside `db_dir`.
    ///
    /// Useful for confirming that a cluster directory is non-empty without
    /// holding a [`PgAccess`] instance.
    ///
    /// # Arguments
    ///
    /// * `db_dir` — The cluster data directory to inspect.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ReadFileError`] if the filesystem check fails.
    pub async fn pg_version_file_exists(db_dir: &Path) -> Result<bool> {
        let pg_version_file = db_dir.join(PG_VERSION_FILE_NAME);
        Self::path_exists(&pg_version_file).await
    }

    /// Returns `true` if `file` exists on the filesystem.
    ///
    /// Uses [`tokio::fs::try_exists`] which returns `false` (not an error) for
    /// permission-denied on the file itself; see its documentation for edge
    /// cases.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ReadFileError`] if the syscall itself fails (e.g.
    /// the parent directory is inaccessible).
    async fn path_exists(file: &Path) -> Result<bool> {
        tokio::fs::try_exists(file)
            .await
            .map_err(|e| Error::ReadFileError(e.to_string()))
    }

    /// Returns the current acquisition status for this instance's cache directory.
    pub async fn acquisition_status(&self) -> PgAcquisitionStatus {
        let lock = ACQUIRED_PG_BINS.lock().await;
        let acquisition_status = lock.get(&self.cache_dir);
        match acquisition_status {
            None => PgAcquisitionStatus::Undefined,
            Some(status) => *status,
        }
    }

    /// Removes the database cluster directory and the password file.
    ///
    /// Both removals are attempted even if the first one fails; the first
    /// error encountered is returned.  Called synchronously from
    /// [`crate::postgres::PgEmbed`]'s `Drop` implementation.
    ///
    /// # Errors
    ///
    /// Returns [`Error::PgCleanUpFailure`] if either removal fails.
    pub fn clean(&self) -> Result<()> {
        let dir_result = std::fs::remove_dir_all(&self.database_dir)
            .map_err(|e| Error::PgCleanUpFailure(e.to_string()));
        let file_result = std::fs::remove_file(&self.pw_file_path)
            .map_err(|e| Error::PgCleanUpFailure(e.to_string()));
        // Both operations run before returning the first error (if any)
        dir_result.and(file_result)
    }

    /// Removes the entire `pg-embed` binary cache directory.
    ///
    /// Useful for freeing disk space or forcing a fresh download.  Errors
    /// during removal are silently ignored (the function always returns `Ok`).
    ///
    /// # Errors
    ///
    /// Returns [`Error::ReadFileError`] if the OS cache directory cannot be
    /// resolved.
    pub async fn purge() -> Result<()> {
        let mut cache_dir = dirs::cache_dir()
            .ok_or_else(|| Error::ReadFileError("cache dir not found".into()))?;
        cache_dir.push(PG_EMBED_CACHE_DIR_NAME);
        let _ = tokio::fs::remove_dir_all(&cache_dir).await;
        Ok(())
    }

    /// Removes `database_dir` and `pw_file` asynchronously.
    ///
    /// Unlike [`Self::clean`], this is an `async` free-standing helper and
    /// stops on the first error.
    ///
    /// # Arguments
    ///
    /// * `database_dir` — The cluster data directory to remove.
    /// * `pw_file` — The password file to remove.
    ///
    /// # Errors
    ///
    /// Returns [`Error::PgCleanUpFailure`] if either removal fails.
    pub async fn clean_up(database_dir: PathBuf, pw_file: PathBuf) -> Result<()> {
        tokio::fs::remove_dir_all(&database_dir)
            .await
            .map_err(|e| Error::PgCleanUpFailure(e.to_string()))?;

        tokio::fs::remove_file(&pw_file)
            .await
            .map_err(|e| Error::PgCleanUpFailure(e.to_string()))
    }

    /// Writes `password` bytes to [`Self::pw_file_path`].
    ///
    /// `initdb` reads this file via `--pwfile` to set the superuser password
    /// without exposing it on the command line.
    ///
    /// # Arguments
    ///
    /// * `password` — The password bytes to write (UTF-8 text is expected but
    ///   not enforced).
    ///
    /// # Errors
    ///
    /// Returns [`Error::WriteFileError`] if the file cannot be created or the
    /// write fails.
    pub async fn create_password_file(&self, password: &[u8]) -> Result<()> {
        let mut file = tokio::fs::File::create(self.pw_file_path.as_path())
            .await
            .map_err(|e| Error::WriteFileError(e.to_string()))?;
        file.write_all(password)
            .await
            .map_err(|e| Error::WriteFileError(e.to_string()))
    }

    /// Installs a third-party extension into the binary cache.
    ///
    /// Copies files from `extension_dir` into the appropriate subdirectory of
    /// [`Self::cache_dir`]:
    ///
    /// | Source extension | Destination |
    /// |---|---|
    /// | `.so`, `.dylib`, `.dll` | `{cache_dir}/lib/` |
    /// | `.control`, `.sql` | `{cache_dir}/share/postgresql/extension/` (or equivalent) |
    /// | anything else, subdirectories | silently skipped |
    ///
    /// Call this method after [`crate::postgres::PgEmbed::setup`] and before
    /// [`crate::postgres::PgEmbed::start_db`], then run
    /// `CREATE EXTENSION IF NOT EXISTS <name>` once the server is up.
    ///
    /// # Arguments
    ///
    /// * `extension_dir` — Directory containing the extension files to install.
    ///
    /// # Errors
    ///
    /// Returns [`Error::DirCreationError`] if the target directories cannot be
    /// created.
    /// Returns [`Error::ReadFileError`] if `extension_dir` cannot be read or a
    /// directory entry cannot be inspected.
    /// Returns [`Error::WriteFileError`] if a file cannot be copied.
    /// Returns the path of the `extension/` directory inside the binary cache.
    ///
    /// Searches for an existing `extension/` subdirectory under `share/` in the
    /// cache (trying common PostgreSQL layout variants).  Falls back to
    /// `share/postgresql/extension` — the standard location used by the
    /// zonkyio binaries — when none of the candidates exist yet.
    async fn share_extension_dir(cache_dir: &Path) -> PathBuf {
        let candidates = [
            cache_dir.join("share/postgresql/extension"),
            cache_dir.join("share/extension"),
        ];
        for candidate in &candidates {
            if tokio::fs::try_exists(candidate).await.unwrap_or(false) {
                return candidate.clone();
            }
        }
        candidates[0].clone()
    }

    pub async fn install_extension(&self, extension_dir: &Path) -> Result<()> {
        let lib_dir = self.cache_dir.join("lib");
        let share_ext_dir = Self::share_extension_dir(&self.cache_dir).await;

        tokio::fs::create_dir_all(&lib_dir)
            .await
            .map_err(|e| Error::DirCreationError(e.to_string()))?;
        tokio::fs::create_dir_all(&share_ext_dir)
            .await
            .map_err(|e| Error::DirCreationError(e.to_string()))?;

        let mut entries = tokio::fs::read_dir(extension_dir)
            .await
            .map_err(|e| Error::ReadFileError(e.to_string()))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| Error::ReadFileError(e.to_string()))?
        {
            let file_type = entry
                .file_type()
                .await
                .map_err(|e| Error::ReadFileError(e.to_string()))?;
            if !file_type.is_file() {
                continue;
            }

            let path = entry.path();
            let file_name = match path.file_name() {
                Some(n) => n,
                None => continue,
            };
            let dest_dir = match path.extension().and_then(|e| e.to_str()) {
                Some("so") | Some("dylib") | Some("dll") => &lib_dir,
                Some("control") | Some("sql") => &share_ext_dir,
                _ => continue,
            };
            tokio::fs::copy(&path, dest_dir.join(file_name))
                .await
                .map_err(|e| Error::WriteFileError(e.to_string()))?;
        }
        Ok(())
    }

    /// Builds a synchronous `pg_ctl stop` [`std::process::Command`].
    ///
    /// Uses [`OsStr`][std::ffi::OsStr] arguments throughout to avoid UTF-8
    /// conversion failures on platforms with non-Unicode paths.  The returned
    /// [`PgCommandSync`] is ready to be spawned but has not yet been started.
    ///
    /// # Arguments
    ///
    /// * `database_dir` — Passed as the `-D` argument to `pg_ctl stop`.
    pub fn stop_db_command_sync(&self, database_dir: &Path) -> PgCommandSync {
        let mut command = Box::new(Cell::new(
            std::process::Command::new(self.pg_ctl_exe.as_os_str()),
        ));
        command.get_mut().arg("stop").arg("-w").arg("-D").arg(database_dir);
        command
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pg_fetch::{PgFetchSettings, PG_V17};

    #[tokio::test]
    async fn test_install_extension() {
        let src_dir = tempfile::TempDir::new().unwrap();
        let src_path = src_dir.path();

        std::fs::write(src_path.join("myvec.so"), b"fake so").unwrap();
        std::fs::write(src_path.join("myvec.dylib"), b"fake dylib").unwrap();
        std::fs::write(src_path.join("myvec.control"), b"# control").unwrap();
        std::fs::write(src_path.join("myvec--1.0.sql"), b"-- sql").unwrap();
        std::fs::write(src_path.join("README.txt"), b"readme").unwrap();

        let cache_dir = tempfile::TempDir::new().unwrap();
        let cache_path = cache_dir.path().to_path_buf();

        let pg_access = PgAccess {
            cache_dir: cache_path.clone(),
            database_dir: cache_path.join("db"),
            pg_ctl_exe: cache_path.join("bin/pg_ctl"),
            init_db_exe: cache_path.join("bin/initdb"),
            pw_file_path: cache_path.join("db.pwfile"),
            zip_file_path: cache_path.join("pg.zip"),
            pg_version_file: cache_path.join("db/PG_VERSION"),
            fetch_settings: PgFetchSettings {
                version: PG_V17,
                ..Default::default()
            },
        };

        pg_access.install_extension(src_path).await.unwrap();

        assert!(cache_path.join("lib/myvec.so").exists(), "lib/myvec.so missing");
        assert!(cache_path.join("lib/myvec.dylib").exists(), "lib/myvec.dylib missing");
        // No existing share dir → falls back to share/postgresql/extension
        assert!(
            cache_path.join("share/postgresql/extension/myvec.control").exists(),
            "share/postgresql/extension/myvec.control missing"
        );
        assert!(
            cache_path.join("share/postgresql/extension/myvec--1.0.sql").exists(),
            "share/postgresql/extension/myvec--1.0.sql missing"
        );
        assert!(
            !cache_path.join("lib/README.txt").exists(),
            "README.txt should not be in lib/"
        );
        assert!(
            !cache_path.join("share/postgresql/extension/README.txt").exists(),
            "README.txt should not be in share/postgresql/extension/"
        );
    }
}
