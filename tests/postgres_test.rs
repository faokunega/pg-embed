use pg_embed::fetch;
use pg_embed::errors::PgEmbedError;
use std::time::Instant;
use sqlx::{Connection, PgConnection};
use serial_test::serial;

mod common;

#[tokio::test]
#[serial]
async fn postgres_server_start_stop() -> Result<(), PgEmbedError> {
    let mut pg = common::setup().await?;
    pg.start_db().await?;
    let _ = pg.stop_db()?;
    Ok(())
}

#[tokio::test]
#[serial]
async fn db_creation() -> Result<(), PgEmbedError> {
    let mut pg = common::setup().await?;
    pg.start_db().await?;
    let db_name = "test";
    pg.create_database(&db_name).await?;
    let mut conn = PgConnection::connect(&pg.full_db_uri("test")).await?;
    sqlx::query("CREATE TABLE IF NOT EXISTS tags(id SERIAL, name VARCHAR(100) NOT NULL)")
        .execute(&mut conn).await?;

    Ok(())
}