use thiserror::Error;

///
/// PgEmbed errors when using feature = "rt_tokio"
///
#[derive(Error, Debug)]
pub enum PgEmbedError {
    /// Postgresql binaries download failure
    #[error("postgresql binaries download failure")]
    DownloadFailure(reqwest::Error),
    /// Request response bytes convertion failure
    #[error("conversion failure")]
    ConversionFailure(reqwest::Error),
}
