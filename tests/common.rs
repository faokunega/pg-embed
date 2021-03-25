use pg_embed::postgres::{PgEmbed, PgSettings};
use pg_embed::fetch;
use pg_embed::fetch::{OperationSystem, Architecture, FetchSettings, PG_V13};
use pg_embed::errors::PgEmbedError;
use std::time::Duration;

pub async fn setup() -> Result<PgEmbed, PgEmbedError> {
    let pg_settings = PgSettings{
        executables_dir: "data_test/postgres".to_string(),
        database_dir: "data_test/db".to_string(),
        port: 5432,
        user: "postgres".to_string(),
        password: "password".to_string(),
        persistent: false,
        start_timeout: Duration::from_secs(5),
        migration_dir: None,
    };
    let fetch_settings = FetchSettings{
        host: "https://repo1.maven.org".to_string(),
        operating_system: OperationSystem::Darwin,
        architecture: Architecture::Amd64,
        version: PG_V13
    };
    let pg = PgEmbed::new(pg_settings, fetch_settings);
    pg.setup().await?;
    Ok(pg)
}