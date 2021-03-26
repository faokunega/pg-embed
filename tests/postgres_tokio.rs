use pg_embed::errors::PgEmbedError;
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
