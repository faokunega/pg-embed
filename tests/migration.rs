use std::path::PathBuf;

use serial_test::file_serial;
use tempfile::TempDir;

use pg_embed::pg_errors::{Error, Result};
use sqlx::{Connection, PgConnection};

#[path = "common.rs"]
mod common;

#[tokio::test]
#[file_serial(pg_port_5432)]
async fn migration() -> Result<()> {
    let (_dir, mut pg) = common::setup_with_tempdir(
        5432,
        false,
        Some(PathBuf::from("migration_test")),
    )
    .await?;
    pg.start_db().await?;
    pg.create_database("test").await?;
    pg.migrate("test").await?;

    let mut conn = PgConnection::connect(&pg.full_db_uri("test"))
        .await
        .map_err(|e| Error::SqlQueryError(e.to_string()))?;

    let _ = sqlx::query("INSERT INTO testing (description) VALUES ('Hello')")
        .execute(&mut conn)
        .await
        .map_err(|e| Error::SqlQueryError(e.to_string()))?;

    let rows = sqlx::query("SELECT * FROM testing")
        .fetch_all(&mut conn)
        .await
        .map_err(|e| Error::SqlQueryError(e.to_string()))?;

    assert_eq!(1, rows.len());
    Ok(())
}

/// Verify that `migrate()` is a no-op (returns `Ok`) when `migration_dir` is `None`.
#[tokio::test]
#[file_serial(pg_port_5432)]
async fn migrate_no_dir() -> Result<()> {
    let (_dir, mut pg) = common::setup_with_tempdir(5432, false, None).await?;
    pg.start_db().await?;
    pg.create_database("test_nodir").await?;
    pg.migrate("test_nodir").await?;
    Ok(())
}

/// Verify that `migrate()` returns `SqlQueryError` when the target database does not exist.
#[tokio::test]
#[file_serial(pg_port_5432)]
async fn migrate_nonexistent_database() -> Result<()> {
    let (_dir, mut pg) = common::setup_with_tempdir(
        5432,
        false,
        Some(PathBuf::from("migration_test")),
    )
    .await?;
    pg.start_db().await?;
    // Do NOT create the database — pool.connect() should fail
    let result = pg.migrate("ghost_db_xyz").await;
    assert!(matches!(result, Err(Error::SqlQueryError(_))));
    Ok(())
}

/// Verify that a migration file containing invalid SQL returns `MigrationError`.
#[tokio::test]
#[file_serial(pg_port_5432)]
async fn migration_invalid_sql() -> Result<()> {
    // migration_dir declared first so it outlives pg
    let migration_dir = TempDir::new().map_err(|e| Error::DirCreationError(e.to_string()))?;
    std::fs::write(
        migration_dir.path().join("1_bad.sql"),
        "SELECT * FROM table_that_does_not_exist_xyz;",
    )
    .map_err(|e| Error::WriteFileError(e.to_string()))?;

    let (_dir, mut pg) = common::setup_with_tempdir(
        5432,
        false,
        Some(migration_dir.path().to_path_buf()),
    )
    .await?;
    pg.start_db().await?;
    pg.create_database("test_bad_sql").await?;

    let result = pg.migrate("test_bad_sql").await;
    assert!(matches!(result, Err(Error::MigrationError(_))));
    Ok(())
}
