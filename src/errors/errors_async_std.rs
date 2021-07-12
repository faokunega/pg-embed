use thiserror::Error;

///
/// PgEmbed errors when using feature = "rt_async_std"
///
#[derive(Error, Debug)]
pub enum PgEmbedErrorExt {
    /// Postgresql binaries download failure
    #[error("postgresql binaries download failure")]
    DownloadFailure(surf::Error),
    /// Request response bytes convertion failure
    #[error("conversion failure")]
    ConversionFailure(surf::Error),
}
