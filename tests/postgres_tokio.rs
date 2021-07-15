use pg_embed::pg_errors::PgEmbedError;
use serial_test::serial;
use std::path::PathBuf;
use pg_embed::pg_enums::PgServerStatus;
use pg_embed::pg_access::PgAccess;
use futures::TryFutureExt;
use pg_embed::postgres::PgEmbed;
use std::borrow::BorrowMut;

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
    PgAccess::purge().await?;
    // let mut pg1 = common::setup(5432, PathBuf::from("data_test/db1"), false, None).await?;
    // let mut pg2 = common::setup(5433, PathBuf::from("data_test/db2"), false, None).await?;
    // let mut pg3 = common::setup(5434, PathBuf::from("data_test/db3"), false, None).await?;
    let tasks = vec![
        tokio::spawn(async move { common::setup(5432, PathBuf::from("data_test/db1"), false, None).await }),
        tokio::spawn(async move { common::setup(5433, PathBuf::from("data_test/db2"), false, None).await }),
        tokio::spawn(async move { common::setup(5434, PathBuf::from("data_test/db3"), false, None).await }),
    ];

    let mut pgs: Vec<PgEmbed> =
        futures::future::join_all(tasks)
            .await
            .into_iter()
            .map(|val| val.map_err(|e| PgEmbedError::PgLockError()).unwrap().unwrap())
            .collect();

    pgs.get_mut(0).unwrap().start_db().await?;
    assert_eq!(pgs.get(0).unwrap().server_status, PgServerStatus::Started);
    pgs.get_mut(1).unwrap().start_db().await?;
    assert_eq!(pgs.get(1).unwrap().server_status, PgServerStatus::Started);
    pgs.get_mut(2).unwrap().start_db().await?;
    assert_eq!(pgs.get(2).unwrap().server_status, PgServerStatus::Started);
    pgs.get_mut(0).unwrap().stop_db().await?;
    assert_eq!(pgs.get(0).unwrap().server_status, PgServerStatus::Stopped);
    pgs.get_mut(1).unwrap().stop_db().await?;
    assert_eq!(pgs.get(1).unwrap().server_status, PgServerStatus::Stopped);
    pgs.get_mut(2).unwrap().stop_db().await?;
    assert_eq!(pgs.get(2).unwrap().server_status, PgServerStatus::Stopped);
    Ok(())
}

#[tokio::test]
#[serial]
async fn postgres_server_persistent_true() -> Result<(), PgEmbedError> {
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

#[tokio::test]
#[serial]
async fn postgres_server_persistent_false() -> Result<(), PgEmbedError> {
    let mut db_path = PathBuf::from("data_test/db");
    {
        let mut pg = common::setup(
            5432,
            db_path.clone(),
            false,
            None,
        ).await?;
        let file_exists = common::pg_version_file_exists(&db_path).await?;
        assert_eq!(true, file_exists);
    }
    let file_exists = common::pg_version_file_exists(&db_path).await?;
    assert_eq!(false, file_exists);

    Ok(())
}
