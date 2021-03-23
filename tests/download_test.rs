use pg_embed::fetch;

mod common;

#[async_std::test]
async fn postgres_pwfile_creation() -> anyhow::Result<()>{
    let pg_embed = common::setup();
    pg_embed.create_password_file().await
}

#[async_std::test]
async fn postgres_initialization() -> anyhow::Result<()>{
    let pg_embed = common::setup();
    let mut child_process = pg_embed.init_db().await?;
    Ok(())
}

#[async_std::test]
async fn postgres_server_start() -> anyhow::Result<()>{
    let mut pg_embed = common::setup();
    pg_embed.start_db().await?;
    pg_embed.process.map(|mut p| p.kill().unwrap());
    Ok(())
}

#[async_std::test]
async fn postgres_server_stop() -> anyhow::Result<()>{
    let mut pg_embed = common::setup();
    let _ = pg_embed.start_db().await?;
    let duration = std::time::Duration::from_secs(10);
    std::thread::sleep(duration);
    let _ = pg_embed.stop_db().await?;
    Ok(())
}

// #[async_std::test]
// async fn postgres_download() -> anyhow::Result<()>{
//     let pg_embed = common::setup();
//     pg_embed.aquire_postgres().await
// }

// #[async_std::test]
// async fn postgres_unpacking(
// ) -> anyhow::Result<()> {
//     let pg_file = "darwin-amd64-13.1.0-1.zip";
//     let executables_dir = "data/postgres";
//     fetch::unpack_postgres(&pg_file, &executables_dir).await
// }
