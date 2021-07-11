use thiserror::Error;

///
/// PgEmbed errors when using feature = "rt_actix_migrate"
///
#[derive(Error, Debug)]
pub enum PgEmbedError {
    /// Postgresql binaries download failure
    #[error("postgresql binaries download failure")]
    DownloadFailure(reqwest::Error),
    /// Request response bytes convertion failure
    #[error("conversion failure")]
    ConversionFailure(reqwest::Error),
    /// sqlx query error
    #[error("query error")]
    SqlQueryError(#[from] sqlx_actix::Error),
    /// migration error
    #[error("migration error")]
    MigrationError(#[from] sqlx_actix::migrate::MigrateError),
}
