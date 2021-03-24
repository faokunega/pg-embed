use thiserror::Error;

///
/// PgEmbed errors
///
#[derive(Error, Debug)]
pub enum PgEmbedError {
    #[error("invalid postgresql binaries download url: `{0}`")]
    InvalidPgUrl(String),
    #[error("invalid postgresql binaries package. `{0}`")]
    InvalidPgPackage(String),
    #[error("postgresql binaries download failure")]
    DownloadFailure(surf::Error),
    #[error("could not write to file")]
    WriteFileError(std::io::Error),
    #[error("could not read file")]
    ReadFileError(std::io::Error),
    #[error("could not create directory")]
    DirCreationError(std::io::Error),
    #[error("conversion failure")]
    ConversionFailure(surf::Error),
    #[error("failed to unpack postgresql binaries`")]
    UnpackFailure(#[from] archiver_rs::ArchiverError),
    #[error("postgresql could not be started")]
    PgStartFailure(std::io::Error),
    #[error("postgresql could not be stopped")]
    PgStopFailure(std::io::Error),
    #[error("postgresql could not be initialized")]
    PgInitFailure(std::io::Error),
}