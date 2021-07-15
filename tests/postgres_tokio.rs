use pg_embed::pg_errors::PgEmbedError;
use serial_test::serial;
use std::path::PathBuf;
use pg_embed::pg_enums::PgServerStatus;

mod common;

#[tokio::test]
#[serial]
async fn postgres_server_start_stop() -> Result<(), PgEmbedError> {
    let mut pg = common::setup(5432, PathBuf::from("data_test/db"), false, None).await?;
    assert_eq!(pg.server_status, PgServerStatus::Initialized);
    pg.start_db().await?;
    assert_eq!(pg.server_status, PgServerStatus::Started);
    pg.stop_db().await?;
    assert_eq!(pg.server_status, PgServerStatus::Stopped);
    Ok(())
}

#[tokio::test]
#[serial]
async fn postgres_server_drop() -> Result<(), PgEmbedError> {
    let mut db_path = PathBuf::from("data_test/db");
    {
        let mut pg = common::setup(5432, db_path.clone(), false, None).await?;
        pg.start_db().await?;
        let file_exists = common::pg_version_file_exists(&db_path).await?;
        assert_eq!(true, file_exists);
    }
    let file_exists = common::pg_version_file_exists(&db_path).await?;
    assert_eq!(false, file_exists);
    Ok(())
}

#[tokio::test]
#[serial]
async fn postgres_server_multiple_concurrent() -> Result<(), PgEmbedError> {
    let mut pg1 = common::setup(5432, PathBuf::from("data_test/db1"), false, None).await?;
    let mut pg2 = common::setup(5433, PathBuf::from("data_test/db2"), false, None).await?;
    let mut pg3 = common::setup(5434, PathBuf::from("data_test/db3"), false, None).await?;
    pg1.start_db().await?;
    assert_eq!(pg1.server_status, PgServerStatus::Started);
    pg2.start_db().await?;
    assert_eq!(pg2.server_status, PgServerStatus::Started);
    pg3.start_db().await?;
    assert_eq!(pg3.server_status, PgServerStatus::Started);
    pg1.stop_db().await?;
    assert_eq!(pg1.server_status, PgServerStatus::Stopped);
    pg2.stop_db().await?;
    assert_eq!(pg2.server_status, PgServerStatus::Stopped);
    pg3.stop_db().await?;
    assert_eq!(pg3.server_status, PgServerStatus::Stopped);
    Ok(())
}

#[tokio::test]
#[serial]
async fn postgres_server_persistent() -> Result<(), PgEmbedError> {
    let mut db_path = PathBuf::from("data_test/db");
    let mut database_dir = PathBuf::new();
    let mut pw_file_path = PathBuf::new();
    {
        let mut pg = common::setup(
            5432,
            db_path.clone(),
            true,
            None,
        ).await?;
        database_dir = pg.pg_access.database_dir.clone();
        pw_file_path = pg.pg_access.pw_file_path.clone();
        let file_exists = common::pg_version_file_exists(&db_path).await?;
        assert_eq!(true, file_exists);
    }
    let file_exists = common::pg_version_file_exists(&db_path).await?;
    assert_eq!(true, file_exists);

    common::clean_up(database_dir, pw_file_path).await?;

    let file_exists = common::pg_version_file_exists(&db_path).await?;
    assert_eq!(false, file_exists);

    Ok(())
}
