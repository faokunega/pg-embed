use pg_embed::postgres::{PgEmbed, PgSettings};
use pg_embed::pg_fetch::{PgFetchSettings, PG_V13};
use std::time::Duration;
use std::path::PathBuf;
use std::io::{Error, ErrorKind};
use pg_embed::pg_errors::PgEmbedError;
use pg_embed::pg_enums::PgAuthMethod;

pub async fn setup() -> Result<PgEmbed, PgEmbedError> {
    let pg_settings = PgSettings{
        database_dir: PathBuf::from("data_test/db"),
        port: 5432,
        user: "postgres".to_string(),
        password: "password".to_string(),
        auth_method: PgAuthMethod::MD5,
        persistent: false,
        timeout: Duration::from_secs(60),
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