use std::time::Duration;

use serial_test::file_serial;
use tempfile::TempDir;

use pg_embed::pg_enums::PgAuthMethod;
use pg_embed::pg_errors::{Error, Result};
use pg_embed::pg_fetch::{PgFetchSettings, PG_V17};
use pg_embed::postgres::{PgEmbed, PgSettings};

#[path = "common.rs"]
mod common;

#[tokio::test]
#[file_serial(pg_port_5432)]
async fn create_database() -> Result<()> {
    let (_dir, mut pg) = common::setup_with_tempdir(5432, false, None).await?;
    pg.start_db().await?;

    pg.create_database("test").await?;
    assert!(pg.database_exists("test").await?);
    Ok(())
}

#[tokio::test]
#[file_serial(pg_port_5432)]
async fn drop_database() -> Result<()> {
    let (_dir, mut pg) = common::setup_with_tempdir(5432, false, None).await?;
    pg.start_db().await?;

    pg.create_database("test").await?;
    assert!(pg.database_exists("test").await?);

    pg.drop_database("test").await?;
    assert!(!pg.database_exists("test").await?);
    Ok(())
}

/// Verify that `database_exists` returns `false` for a database that was never created.
#[tokio::test]
#[file_serial(pg_port_5432)]
async fn database_exists_false() -> Result<()> {
    let (_dir, mut pg) = common::setup_with_tempdir(5432, false, None).await?;
    pg.start_db().await?;
    assert!(!pg.database_exists("nonexistent_db_xyz").await?);
    Ok(())
}

/// Verify that creating the same database twice returns an error.
#[tokio::test]
#[file_serial(pg_port_5432)]
async fn create_duplicate() -> Result<()> {
    let (_dir, mut pg) = common::setup_with_tempdir(5432, false, None).await?;
    pg.start_db().await?;
    pg.create_database("dup_test").await?;
    let result = pg.create_database("dup_test").await;
    assert!(result.is_err());
    Ok(())
}

/// Verify that dropping a database that does not exist is a no-op.
///
/// sqlx's `Postgres::drop_database` uses `DROP DATABASE IF EXISTS` semantics,
/// so dropping a non-existent database succeeds silently.
#[tokio::test]
#[file_serial(pg_port_5432)]
async fn drop_nonexistent() -> Result<()> {
    let (_dir, mut pg) = common::setup_with_tempdir(5432, false, None).await?;
    pg.start_db().await?;
    pg.drop_database("this_db_does_not_exist_xyz").await?;
    Ok(())
}

/// Verify the format of `db_uri` and `full_db_uri` without starting a server.
#[tokio::test]
async fn full_uri_format() -> Result<()> {
    let dir = TempDir::new().map_err(|e| Error::DirCreationError(e.to_string()))?;
    let pg_settings = PgSettings {
        database_dir: dir.path().join("db"),
        port: 15432,
        user: "alice".to_string(),
        password: "s3cr3t".to_string(),
        auth_method: PgAuthMethod::MD5,
        persistent: false,
        timeout: Some(Duration::from_secs(10)),
        migration_dir: None,
    };
    let fetch_settings = PgFetchSettings { version: PG_V17, ..Default::default() };
    let pg = PgEmbed::new(pg_settings, fetch_settings).await?;
    assert_eq!(pg.db_uri, "postgres://alice:s3cr3t@localhost:15432");
    assert_eq!(pg.full_db_uri("mydb"), "postgres://alice:s3cr3t@localhost:15432/mydb");
    Ok(())
}
