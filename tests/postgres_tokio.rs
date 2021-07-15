use std::path::PathBuf;

use futures::stream::StreamExt;
use serial_test::serial;
use tokio::sync::Mutex;

use pg_embed::pg_access::PgAccess;
use pg_embed::pg_enums::PgServerStatus;
use pg_embed::pg_errors::PgEmbedError;
use pg_embed::postgres::PgEmbed;

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
    let db_path = PathBuf::from("data_test/db");
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

    let tasks = vec![
        common::setup(5432, PathBuf::from("data_test/db1"), false, None),
        common::setup(5433, PathBuf::from("data_test/db2"), false, None),
        common::setup(5434, PathBuf::from("data_test/db3"), false, None),
    ];

    let wrap_with_mutex =
        |val: Result<PgEmbed, PgEmbedError>|
            val.map(|pg| Mutex::new(pg)).unwrap();

    let pgs: Vec<Mutex<PgEmbed>> =
        futures::future::join_all(tasks)
            .await
            .into_iter()
            .map(wrap_with_mutex)
            .collect();

    futures::stream::iter(&pgs).for_each_concurrent(None, |pg| async move {
        let mut pg = pg.lock().await;
        let _ = pg.start_db().await;
        assert_eq!(pg.server_status, PgServerStatus::Started);
    }).await;

    futures::stream::iter(&pgs).for_each_concurrent(None, |pg| async move {
        let mut pg = pg.lock().await;
        let _ = pg.stop_db().await;
        assert_eq!(pg.server_status, PgServerStatus::Stopped);
    }).await;

    Ok(())
}

#[tokio::test]
#[serial]
async fn postgres_server_persistent_true() -> Result<(), PgEmbedError> {
    let db_path = PathBuf::from("data_test/db");
    let mut database_dir = PathBuf::new();
    let mut pw_file_path = PathBuf::new();
    {
        let pg = common::setup(
            5432,
            db_path.clone(),
            true,
            None,
        ).await?;
        database_dir.clone_from(&pg.pg_access.database_dir);
        pw_file_path.clone_from(&pg.pg_access.pw_file_path);
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
    let db_path = PathBuf::from("data_test/db");
    {
        let _pg = common::setup(
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
