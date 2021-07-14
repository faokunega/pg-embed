use pg_embed::pg_errors::PgEmbedError;
use serial_test::serial;
use std::path::PathBuf;

mod common;

#[tokio::test]
#[serial]
async fn postgres_server_start_stop() -> Result<(), PgEmbedError> {
    let mut pg = common::setup(5432, PathBuf::from("data_test/db")).await?;
    pg.start_db().await?;
    pg.stop_db().await?;
    Ok(())
}

#[tokio::test]
#[serial]
async fn postgres_server_multiple_concurrent() -> Result<(), PgEmbedError> {
    let mut pg1 = common::setup(5432, PathBuf::from("data_test/db1")).await?;
    let mut pg2 = common::setup(5433, PathBuf::from("data_test/db2")).await?;
    let mut pg3 = common::setup(5434, PathBuf::from("data_test/db3")).await?;
    pg1.start_db().await?;
    pg2.start_db().await?;
    pg3.start_db().await?;
    pg1.stop_db().await?;
    pg2.stop_db().await?;
    pg3.stop_db().await?;
    Ok(())
}
