use pg_embed::fetch;
use pg_embed::errors::PgEmbedError;

mod common;

#[tokio::test]
async fn postgres_pwfile_creation() -> Result<(), PgEmbedError>{
    let pg_embed = common::setup();
    pg_embed.create_password_file().await
}

#[tokio::test]
async fn postgres_initialization() -> Result<(), PgEmbedError>{
    let pg_embed = common::setup();
    let mut child_process = pg_embed.init_db().await?;
    Ok(())
}

#[tokio::test]
async fn postgres_server_start() -> Result<(), PgEmbedError>{
    let mut pg_embed = common::setup();
    pg_embed.start_db().await?;
    pg_embed.process.as_mut().map(|p| p.kill().unwrap());
    Ok(())
}

#[tokio::test]
async fn postgres_server_stop() -> Result<(), PgEmbedError>{
    let mut pg_embed = common::setup();
    let _ = pg_embed.start_db().await?;
    let duration = std::time::Duration::from_secs(10);
    std::thread::sleep(duration);
    let _ = pg_embed.stop_db().await?;
    Ok(())
}

#[tokio::test]
async fn postgres_download() -> Result<(), PgEmbedError>{
    let pg_embed = common::setup();
    pg_embed.aquire_postgres().await
}

// #[tokio::test]
// async fn postgres_unpacking(
// ) -> Result<(), PgEmbedError> {
//     let pg_file = "darwin-amd64-13.1.0-1.zip";
//     let executables_dir = "data/postgres";
//     fetch::unpack_postgres(&pg_file, &executables_dir).await
// }
