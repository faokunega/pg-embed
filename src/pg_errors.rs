//!
//! Errors
//!
use std::error::Error;

use std::fmt;
use std::fmt::Formatter;
use thiserror::Error;

///
/// PgEmbed errors
#[derive(Error, Debug)]
pub struct PgEmbedError {
    pub error_type: PgEmbedErrorType,
    pub source: Option<Box<dyn Error + Sync + Send>>,
    pub message: Option<String>,
}

impl fmt::Display for PgEmbedError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "error_type: {:?}\nsource: \n{:?}\nmessage: \n{:?}\n",
            self.error_type, self.source, self.message
        )
    }
}

///
/// Common pg_embed errors, independent from features used
///
#[derive(Debug, PartialEq)]
pub enum PgEmbedErrorType {
    /// Invalid postgresql binaries download url
    InvalidPgUrl,
    /// Invalid postgresql binaries package
    InvalidPgPackage,
    /// Could not write file
    WriteFileError,
    /// Could not read file
    ReadFileError,
    /// Could not create directory
    DirCreationError,
    /// Failed to unpack postgresql binaries
    UnpackFailure,
    /// Postgresql could not be started
    PgStartFailure,
    /// Postgresql could not be stopped
    PgStopFailure,
    /// Postgresql could not be initialized
    PgInitFailure,
    /// Clean up error
    PgCleanUpFailure,
    /// Purging error
    PgPurgeFailure,
    /// Buffer read error
    PgBufferReadError,
    /// Lock error
    PgLockError,
    /// Child process error
    PgProcessError,
    /// Timed out error
    PgTimedOutError,
    /// Task join error
    PgTaskJoinError,
    /// Error wrapper
    PgError,
    /// Postgresql binaries download failure
    DownloadFailure,
    /// Request response bytes convertion failure
    ConversionFailure,
    /// Channel send error
    SendFailure,
    /// sqlx query error
    SqlQueryError,
    /// migration error
    MigrationError,
}
