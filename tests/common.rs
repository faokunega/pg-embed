use pg_embed::postgres::{PgEmbed, PgSettings, PgAuthMethod};
use pg_embed::fetch::{FetchSettings, PG_V13};
use pg_embed::errors::PgEmbedError;
use std::time::Duration;
use std::path::PathBuf;

pub async fn setup() -> Result<PgEmbed, PgEmbedError> {
    let pg_settings = PgSettings{
        executables_dir: PathBuf::from("data_test/postgres"),
        database_dir: PathBuf::from("data_test/db"),
        port: 5432,
        user: "postgres".to_string(),
        password: "password".to_string(),
        auth_method: PgAuthMethod::MD5,
        persistent: false,
        timeout: Duration::from_secs(5),
        migration_dir: None,
    };
    let fetch_settings = FetchSettings{
        version: PG_V13,
        ..Default::default()
    };
    let pg = PgEmbed::new(pg_settings, fetch_settings);
    pg.setup().await?;
    Ok(pg)
}