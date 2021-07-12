use thiserror::Error;

///
/// PgEmbed errors when using feature = "rt_tokio_migrate"
///
#[derive(Error, Debug)]
pub enum PgEmbedErrorExt {
    /// Postgresql binaries download failure
    #[error("postgresql binaries download failure")]
    DownloadFailure(reqwest::Error),
    /// Request response bytes convertion failure
    #[error("conversion failure")]
    ConversionFailure(reqwest::Error),
    /// sqlx query error
    #[error("query error")]
    SqlQueryError(#[from] sqlx_tokio::Error),
    /// migration error
    #[error("migration error")]
    MigrationError(#[from] sqlx_tokio::migrate::MigrateError),
}
