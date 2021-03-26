use pg_embed::errors::PgEmbedError;
#[cfg(feature = "sqlx_tokio")]
use sqlx_tokio::{Connection, PgConnection};
#[cfg(feature = "sqlx_async_std")]
use sqlx_async_std::{Connection, PgConnection};
#[cfg(feature = "sqlx_actix")]
use sqlx_actix::{Connection, PgConnection};
use serial_test::serial;

mod common;

#[tokio::test]
#[serial]
async fn db_creation() -> Result<(), PgEmbedError> {
    let mut pg = common::setup().await?;
    pg.start_db().await?;
    let db_name = "test";
    pg.create_database(&db_name).await?;
    assert!(pg.database_exists(&db_name).await?);
    Ok(())
}

#[tokio::test]
#[serial]
async fn db_migration() -> Result<(), PgEmbedError> {
    let mut pg = common::setup().await?;
    pg.start_db().await?;
    let db_name = "test";
    pg.create_database(&db_name).await?;
    assert!(pg.database_exists(&db_name).await?);
    pg.drop_database(&db_name).await?;
    assert!(!pg.database_exists(&db_name).await?);
    // pg.migrate(&db_name).await?;
    Ok(())
}
