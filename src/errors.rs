//!
//! Library errors
//!
//! Errors thrown by **pg-embed**
//!
use thiserror::Error;



#[cfg(feature = "rt_tokio")]
#[derive(Error, Debug)]
pub enum PgEmbedError {
    /// Invalid postgresql binaries download url
    #[error("invalid postgresql binaries download url: `{0}`")]
    InvalidPgUrl(String),
    /// Invalid postgresql binaries package
    #[error("invalid postgresql binaries package. `{0}`")]
    InvalidPgPackage(String),
    /// Postgresql binaries download failure
    #[error("postgresql binaries download failure")]
    DownloadFailure(reqwest::Error),
    /// Could not write file
    #[error("could not write file")]
    WriteFileError(std::io::Error),
    /// Could not read file
    #[error("could not read file")]
    ReadFileError(std::io::Error),
    /// Could not create directory
    #[error("could not create directory")]
    DirCreationError(std::io::Error),
    /// Request response bytes convertion failure
    #[error("conversion failure")]
    ConversionFailure(reqwest::Error),
    /// Failed to unpack postgresql binaries
    #[error("failed to unpack postgresql binaries`")]
    UnpackFailure(#[from] archiver_rs::ArchiverError),
    /// Postgresql could not be started
    #[error("postgresql could not be started")]
    PgStartFailure(std::io::Error),
    /// Postgresql could not be stopped
    #[error("postgresql could not be stopped")]
    PgStopFailure(std::io::Error),
    /// Postgresql could not be initialized
    #[error("postgresql could not be initialized")]
    PgInitFailure(std::io::Error),
    /// Clean up error
    #[error("clean up error")]
    PgCleanUpFailure(std::io::Error),
}

///
/// PgEmbed errors
///
#[cfg(feature = "rt_tokio_migrate")]
#[derive(Error, Debug)]
pub enum PgEmbedError {
    /// Invalid postgresql binaries download url
    #[error("invalid postgresql binaries download url: `{0}`")]
    InvalidPgUrl(String),
    /// Invalid postgresql binaries package
    #[error("invalid postgresql binaries package. `{0}`")]
    InvalidPgPackage(String),
    /// Postgresql binaries download failure
    #[error("postgresql binaries download failure")]
    DownloadFailure(reqwest::Error),
    /// Could not write file
    #[error("could not write file")]
    WriteFileError(std::io::Error),
    /// Could not read file
    #[error("could not read file")]
    ReadFileError(std::io::Error),
    /// Could not create directory
    #[error("could not create directory")]
    DirCreationError(std::io::Error),
    /// Request response bytes convertion failure
    #[error("conversion failure")]
    ConversionFailure(reqwest::Error),
    /// Failed to unpack postgresql binaries
    #[error("failed to unpack postgresql binaries`")]
    UnpackFailure(#[from] archiver_rs::ArchiverError),
    /// Postgresql could not be started
    #[error("postgresql could not be started")]
    PgStartFailure(std::io::Error),
    /// Postgresql could not be stopped
    #[error("postgresql could not be stopped")]
    PgStopFailure(std::io::Error),
    /// Postgresql could not be initialized
    #[error("postgresql could not be initialized")]
    PgInitFailure(std::io::Error),
    /// Clean up error
    #[error("clean up error")]
    PgCleanUpFailure(std::io::Error),
    /// sqlx query error
    #[error("query error")]
    SqlQueryError(#[from] sqlx_tokio::Error),
    /// migration error
    #[error("migration error")]
    MigrationError(#[from] sqlx_tokio::migrate::MigrateError),
}
///
/// PgEmbed errors
///
#[cfg(any(feature = "rt_async_std", feature = "rt_async_std_migrate"))]
#[derive(Error, Debug)]
pub enum PgEmbedError {
    /// Invalid postgresql binaries download url
    #[error("invalid postgresql binaries download url: `{0}`")]
    InvalidPgUrl(String),
    /// Invalid postgresql binaries package
    #[error("invalid postgresql binaries package. `{0}`")]
    InvalidPgPackage(String),
    /// Postgresql binaries download failure
    #[error("postgresql binaries download failure")]
    DownloadFailure(reqwest::Error),
    /// Could not write file
    #[error("could not write file")]
    WriteFileError(std::io::Error),
    /// Could not read file
    #[error("could not read file")]
    ReadFileError(std::io::Error),
    /// Could not create directory
    #[error("could not create directory")]
    DirCreationError(std::io::Error),
    /// Request response bytes convertion failure
    #[error("conversion failure")]
    ConversionFailure(surf::Error),
    /// Failed to unpack postgresql binaries
    #[error("failed to unpack postgresql binaries`")]
    UnpackFailure(#[from] archiver_rs::ArchiverError),
    /// Postgresql could not be started
    #[error("postgresql could not be started")]
    PgStartFailure(std::io::Error),
    /// Postgresql could not be stopped
    #[error("postgresql could not be stopped")]
    PgStopFailure(std::io::Error),
    /// Postgresql could not be initialized
    #[error("postgresql could not be initialized")]
    PgInitFailure(std::io::Error),
    /// Clean up error
    #[error("clean up error")]
    PgCleanUpFailure(std::io::Error),
    /// sqlx query error
    #[error("query error")]
    SqlQueryError(#[from] sqlx_async_std::Error),
    /// migration error
    #[error("migration error")]
    MigrationError(#[from] sqlx_async_std::migrate::MigrateError),
}
///
/// PgEmbed errors
///
#[cfg(any(feature = "rt_actix", feature = "rt_actix_migrate"))]
#[derive(Error, Debug)]
pub enum PgEmbedError {
    /// Invalid postgresql binaries download url
    #[error("invalid postgresql binaries download url: `{0}`")]
    InvalidPgUrl(String),
    /// Invalid postgresql binaries package
    #[error("invalid postgresql binaries package. `{0}`")]
    InvalidPgPackage(String),
    /// Postgresql binaries download failure
    #[error("postgresql binaries download failure")]
    DownloadFailure(reqwest::Error),
    /// Could not write file
    #[error("could not write file")]
    WriteFileError(std::io::Error),
    /// Could not read file
    #[error("could not read file")]
    ReadFileError(std::io::Error),
    /// Could not create directory
    #[error("could not create directory")]
    DirCreationError(std::io::Error),
    /// Request response bytes convertion failure
    #[error("conversion failure")]
    ConversionFailure(reqwest::Error),
    /// Failed to unpack postgresql binaries
    #[error("failed to unpack postgresql binaries`")]
    UnpackFailure(#[from] archiver_rs::ArchiverError),
    /// Postgresql could not be started
    #[error("postgresql could not be started")]
    PgStartFailure(std::io::Error),
    /// Postgresql could not be stopped
    #[error("postgresql could not be stopped")]
    PgStopFailure(std::io::Error),
    /// Postgresql could not be initialized
    #[error("postgresql could not be initialized")]
    PgInitFailure(std::io::Error),
    /// Clean up error
    #[error("clean up error")]
    PgCleanUpFailure(std::io::Error),
    /// sqlx query error
    #[error("query error")]
    SqlQueryError(#[from] sqlx_actix::Error),
    /// migration error
    #[error("migration error")]
    MigrationError(#[from] sqlx_actix::migrate::MigrateError),
}