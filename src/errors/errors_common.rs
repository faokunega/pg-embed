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
    /// Purging error
    #[error("purging error")]
    PgPurgeFailure(std::io::Error),
}
