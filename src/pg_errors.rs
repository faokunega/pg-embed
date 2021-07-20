//!
//! Errors
//!
use thiserror::Error;

///
/// Common pg_embed errors, independent from features used
///
#[derive(Error, Debug)]
pub enum PgEmbedError {
    /// Invalid postgresql binaries download url
    #[error("invalid postgresql binaries download url: `{0}`")]
    InvalidPgUrl(String),
    /// Invalid postgresql binaries package
    #[error("invalid postgresql binaries package. `{0}`")]
    InvalidPgPackage(String),
    /// Could not write file
    #[error("could not write file")]
    WriteFileError(std::io::Error),
    /// Could not read file
    #[error("could not read file")]
    ReadFileError(std::io::Error),
    /// Could not create directory
    #[error("could not create directory")]
    DirCreationError(std::io::Error),
    /// Failed to unpack postgresql binaries
    #[error("failed to unpack postgresql binaries`")]
    UnpackFailure(#[from] archiver_rs::ArchiverError),
    /// Postgresql could not be started
    #[error("postgresql could not be started")]
    PgStartFailure(),
    /// Postgresql could not be stopped
    #[error("postgresql could not be stopped")]
    PgStopFailure(),
    /// Postgresql could not be initialized
    #[error("postgresql could not be initialized")]
    PgInitFailure(),
    /// Clean up error
    #[error("clean up error")]
    PgCleanUpFailure(std::io::Error),
    /// Purging error
    #[error("purging error")]
    PgPurgeFailure(std::io::Error),
    /// Buffer read error
    #[error("buffer read error")]
    PgBufferReadError(std::io::Error),
    /// Lock error
    #[error("lock error")]
    PgLockError(),
    /// Child process error
    #[error("process error")]
    PgProcessError(std::io::Error),
    /// Timed out error
    #[error("timed out error")]
    PgTimedOutError(),
    /// Task join error
    #[error("task join error")]
    PgTaskJoinError(),
    /// Error wrapper
    #[error("error wrapper")]
    PgError(#[from] dyn std::error::Error),
    /// Postgresql binaries download failure
    #[error("postgresql binaries download failure")]
    #[cfg(any(
        feature = "rt_tokio",
        feature = "rt_tokio_migrate",
        feature = "rt_actix",
        feature = "rt_actix_migrate"
    ))]
    DownloadFailure(reqwest::Error),
    /// Request response bytes convertion failure
    #[error("conversion failure")]
    #[cfg(any(
        feature = "rt_tokio",
        feature = "rt_tokio_migrate",
        feature = "rt_actix",
        feature = "rt_actix_migrate"
    ))]
    ConversionFailure(reqwest::Error),
    /// Channel send error
    #[error("channel send error")]
    #[cfg(any(
        feature = "rt_tokio",
        feature = "rt_tokio_migrate",
        feature = "rt_actix",
        feature = "rt_actix_migrate"
    ))]
    SendFailure(),
    /// Postgresql binaries download failure
    #[error("postgresql binaries download failure")]
    #[cfg(not(any(
        feature = "rt_tokio",
        feature = "rt_tokio_migrate",
        feature = "rt_actix",
        feature = "rt_actix_migrate"
    )))]
    DownloadFailure(surf::Error),
    /// Request response bytes convertion failure
    #[error("conversion failure")]
    #[cfg(not(any(
        feature = "rt_tokio",
        feature = "rt_tokio_migrate",
        feature = "rt_actix",
        feature = "rt_actix_migrate"
    )))]
    ConversionFailure(surf::Error),
    /// sqlx query error
    #[error("query error")]
    #[cfg(any(feature = "rt_tokio_migrate"))]
    SqlQueryError(#[from] sqlx_tokio::Error),
    /// migration error
    #[error("migration error")]
    #[cfg(any(feature = "rt_tokio_migrate"))]
    MigrationError(#[from] sqlx_tokio::migrate::MigrateError),
    /// sqlx query error
    #[error("query error")]
    #[cfg(not(any(
        feature = "rt_tokio",
        feature = "rt_tokio_migrate",
        feature = "rt_actix",
        feature = "rt_actix_migrate",
        feature = "rt_async_std"
    )))]
    SqlQueryError(#[from] sqlx_async_std::Error),
    /// migration error
    #[error("migration error")]
    #[cfg(not(any(
        feature = "rt_tokio",
        feature = "rt_tokio_migrate",
        feature = "rt_actix",
        feature = "rt_actix_migrate",
        feature = "rt_async_std"
    )))]
    MigrationError(#[from] sqlx_async_std::migrate::MigrateError),
    /// sqlx query error
    #[error("query error")]
    #[cfg(not(any(
        feature = "rt_tokio",
        feature = "rt_tokio_migrate",
        feature = "rt_actix",
        feature = "rt_async_std_migrate",
        feature = "rt_async_std"
    )))]
    SqlQueryError(#[from] sqlx_actix::Error),
    /// migration error
    #[error("migration error")]
    #[cfg(not(any(
        feature = "rt_tokio",
        feature = "rt_tokio_migrate",
        feature = "rt_actix",
        feature = "rt_async_std_migrate",
        feature = "rt_async_std"
    )))]
    MigrationError(#[from] sqlx_actix::migrate::MigrateError),
}
