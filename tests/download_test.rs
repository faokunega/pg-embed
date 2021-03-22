mod common;

#[async_std::test]
async fn postgres_download() -> anyhow::Result<()>{
    let pg_embed = common::setup();
    pg_embed.aquire_postgres().await
}