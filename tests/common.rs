use pg_embed::postgres::{PgEmbed, PgSettings};
use pg_embed::fetch;
use pg_embed::fetch::{OperationSystem, Architecture, FetchSettings, PG_V13};

pub fn setup() -> PgEmbed {
    let pg_settings = PgSettings{
        executables_dir: "data/postgres".to_string(),
        database_dir: "data/db".to_string(),
        user: "postgres".to_string(),
        password: "password".to_string(),
        persistent: false
    };
    let fetch_settings = FetchSettings{
        host: "https://repo1.maven.org".to_string(),
        operating_system: OperationSystem::Darwin,
        architecture: Architecture::Amd64,
        version: PG_V13
    };
    PgEmbed::new(pg_settings, fetch_settings)
}