use thiserror::Error;

///
/// PgEmbed errors when using feature = "rt_async_std_migrate"
///
#[derive(Error, Debug)]
pub enum PgEmbedError {
    /// Postgresql binaries download failure
    #[error("postgresql binaries download failure")]
    DownloadFailure(surf::Error),
    /// Request response bytes convertion failure
    #[error("conversion failure")]
    ConversionFailure(surf::Error),
    /// sqlx query error
    #[error("query error")]
    SqlQueryError(#[from] sqlx_async_std::Error),
    /// migration error
    #[error("migration error")]
    MigrationError(#[from] sqlx_async_std::migrate::MigrateError),
}
