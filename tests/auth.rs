use serial_test::file_serial;
use tempfile::TempDir;

use pg_embed::pg_enums::{PgAuthMethod, PgServerStatus};
use pg_embed::pg_errors::{Error, Result};
use pg_embed::pg_fetch::{PgFetchSettings, PG_V17};
use pg_embed::postgres::{PgEmbed, PgSettings};

/// Verify that the server starts correctly when `PgAuthMethod::Plain` is used.
#[tokio::test]
#[file_serial(pg_port_5432)]
async fn auth_plain() -> Result<()> {
    let dir = TempDir::new().map_err(|e| Error::DirCreationError(e.to_string()))?;
    let pg_settings = PgSettings {
        database_dir: dir.path().join("db"),
        port: 5432,
        user: "postgres".to_string(),
        password: "password".to_string(),
        auth_method: PgAuthMethod::Plain,
        persistent: false,
        timeout: Some(std::time::Duration::from_secs(30)),
        migration_dir: None,
    };
    let fetch_settings = PgFetchSettings { version: PG_V17, ..Default::default() };
    let mut pg = PgEmbed::new(pg_settings, fetch_settings).await?;
    pg.setup().await?;
    pg.start_db().await?;
    {
        let server_status = *pg.server_status.lock().await;
        assert_eq!(server_status, PgServerStatus::Started);
    }
    pg.stop_db().await?;
    Ok(())
}

/// Verify that the server starts correctly when `PgAuthMethod::ScramSha256` is used.
#[tokio::test]
#[file_serial(pg_port_5432)]
async fn auth_scram() -> Result<()> {
    let dir = TempDir::new().map_err(|e| Error::DirCreationError(e.to_string()))?;
    let pg_settings = PgSettings {
        database_dir: dir.path().join("db"),
        port: 5432,
        user: "postgres".to_string(),
        password: "password".to_string(),
        auth_method: PgAuthMethod::ScramSha256,
        persistent: false,
        timeout: Some(std::time::Duration::from_secs(30)),
        migration_dir: None,
    };
    let fetch_settings = PgFetchSettings { version: PG_V17, ..Default::default() };
    let mut pg = PgEmbed::new(pg_settings, fetch_settings).await?;
    pg.setup().await?;
    pg.start_db().await?;
    {
        let server_status = *pg.server_status.lock().await;
        assert_eq!(server_status, PgServerStatus::Started);
    }
    pg.stop_db().await?;
    Ok(())
}
