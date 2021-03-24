use thiserror::Error;

///
/// PgEmbed errors
///
#[derive(Error, Debug)]
pub enum PgEmbedError {
    #[error("invalid postgresql binaries download url: `{0}`")]
    InvalidPgUrl(String),
    #[error("invalid postgresql binaries package")]
    InvalidPgPackage,
    #[error("postgresql binaries download failure")]
    DownloadFailure,
    #[error("failed to unpack postgresql binaries")]
    UnpackFailure,
    #[error("postgresql not started due to `{0}`")]
    PgStartFailure(String),
    #[error("postgresql not stopped due to `{0}`")]
    PgStopFailure(String),
    #[error("postgresql not initialized due to `{0}`")]
    PgInitFailure(String),
}