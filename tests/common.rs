use pg_embed::postgres::{PgEmbed, PgSettings, PgAuthMethod};
use pg_embed::pg_fetch::{PgFetchSettings, PG_V13};
// these cfg feature settings for PgEmbedError are really convoluted, but getting syntax errors otherwise
#[cfg(not(any(feature = "rt_tokio_migrate", feature = "rt_async_std", feature = "rt_async_std_migrate", feature = "rt_actix", feature = "rt_actix_migrate")))]
use pg_embed::errors::errors_tokio::PgEmbedErrorExt;
#[cfg(feature = "rt_tokio_migrate")]
use pg_embed::errors::errors_tokio_migrate::PgEmbedErrorExt;
#[cfg(not(any(feature = "rt_tokio", feature = "rt_tokio_migrate", feature = "rt_async_std_migrate", feature = "rt_actix", feature = "rt_actix_migrate")))]
use pg_embed::errors::errors_async_std::PgEmbedErrorExt;
#[cfg(not(any(feature = "rt_tokio", feature = "rt_tokio_migrate", feature = "rt_async_std", feature = "rt_actix", feature = "rt_actix_migrate")))]
use pg_embed::errors::errors_async_std_migrate::PgEmbedErrorExt;
#[cfg(not(any(feature = "rt_tokio", feature = "rt_tokio_migrate", feature = "rt_async_std", feature = "rt_async_std_migrate", feature = "rt_actix_migrate")))]
use pg_embed::errors::errors_actix::PgEmbedErrorExt;
#[cfg(not(any(feature = "rt_tokio", feature = "rt_tokio_migrate", feature = "rt_async_std", feature = "rt_async_std_migrate", feature = "rt_actix")))]
use pg_embed::errors::errors_actix_migrate::PgEmbedErrorExt;
use std::time::Duration;
use std::path::PathBuf;
use std::io::{Error, ErrorKind};
use pg_embed::errors::errors_common::PgEmbedError;

pub async fn setup() -> Result<PgEmbed, PgEmbedError> {
    let pg_settings = PgSettings{
        database_dir: PathBuf::from("data_test/db"),
        port: 5432,
        user: "postgres".to_string(),
        password: "password".to_string(),
        auth_method: PgAuthMethod::MD5,
        persistent: true,
        timeout: Duration::from_secs(15),
        migration_dir: None,
    };
    let fetch_settings = PgFetchSettings {
        version: PG_V13,
        ..Default::default()
    };
    let mut pg = PgEmbed::new(pg_settings, fetch_settings).await?;
    pg.setup().await?;
    Ok(pg)
}