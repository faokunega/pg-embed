use pg_embed::postgres::{PgEmbed, PgSettings, PgAuthMethod};
use pg_embed::fetch::{OperationSystem, Architecture, FetchSettings, PG_V13};
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
        auth_method: PgAuthMethod::Plain,
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