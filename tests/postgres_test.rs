use pg_embed::fetch;
use pg_embed::errors::PgEmbedError;

mod common;

#[tokio::test]
async fn postgres_server_start_stop() -> Result<(), PgEmbedError>{
    let mut pg_embed = common::setup().await?;
    let _ = pg_embed.start_db().await?;
    let duration = std::time::Duration::from_secs(10);
    std::thread::sleep(duration);
    let _ = pg_embed.stop_db().await?;
    Ok(())
}