use std::path::PathBuf;

use serial_test::serial;

use pg_embed::pg_errors::{PgEmbedError, PgEmbedErrorType};
use sqlx::{Connection, PgConnection};

#[path = "common.rs"]
mod common;

#[tokio::test]
#[serial]
async fn db_create_database() -> Result<(), PgEmbedError> {
    let mut pg = common::setup(5432, PathBuf::from("data_test/db"), false, None).await?;
    pg.start_db().await?;
    let db_name = "test";

    pg.create_database(&db_name).await?;
    assert!(pg.database_exists(&db_name).await?);
    Ok(())
}

#[tokio::test]
#[serial]
async fn db_drop_database() -> Result<(), PgEmbedError> {
    let mut pg = common::setup(5432, PathBuf::from("data_test/db"), false, None).await?;
    pg.start_db().await?;
    let db_name = "test";

    pg.create_database(&db_name).await?;
    assert_eq!(true, pg.database_exists(&db_name).await?);

    pg.drop_database(&db_name).await?;
    assert_eq!(false, pg.database_exists(&db_name).await?);
    Ok(())
}

#[tokio::test]
#[serial]
async fn db_migration() -> Result<(), PgEmbedError> {
    let mut pg = common::setup(
        5432,
        PathBuf::from("data_test/db"),
        false,
        Some(PathBuf::from("migration_test")),
    )
    .await?;
    pg.start_db().await?;
    let db_name = "test";
    pg.create_database(&db_name).await?;

    pg.migrate(&db_name).await?;

    let db_uri = pg.full_db_uri(&db_name);

    let mut conn = PgConnection::connect(&db_uri)
        .await
        .map_err(|_| PgEmbedError {
            error_type: PgEmbedErrorType::SqlQueryError,
            source: None,
            message: None,
        })?;

    let _ = sqlx::query("INSERT INTO testing (description) VALUES ('Hello')")
        .execute(&mut conn)
        .await
        .map_err(|_| PgEmbedError {
            error_type: PgEmbedErrorType::SqlQueryError,
            source: None,
            message: None,
        })?;

    let rows = sqlx::query("SELECT * FROM testing")
        .fetch_all(&mut conn)
        .await
        .map_err(|_| PgEmbedError {
            error_type: PgEmbedErrorType::SqlQueryError,
            source: None,
            message: None,
        })?;

    assert_eq!(1, rows.len());

    Ok(())
}
