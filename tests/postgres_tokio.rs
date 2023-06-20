use std::path::PathBuf;

use futures::stream::StreamExt;
use serial_test::serial;
use tokio::sync::Mutex;

use env_logger::Env;
use pg_embed::pg_access::PgAccess;
use pg_embed::pg_enums::{PgAuthMethod, PgServerStatus};
use pg_embed::pg_errors::PgEmbedError;
use pg_embed::pg_fetch::{PgFetchSettings, PG_V15};
use pg_embed::postgres::{PgEmbed, PgSettings};
use std::time::Duration;

#[path = "common.rs"]
mod common;

#[tokio::test]
#[serial]
async fn postgres_server_start_stop() -> Result<(), PgEmbedError> {
    let mut pg = common::setup(5432, PathBuf::from("data_test/db"), false, None).await?;
    {
        let server_status = *pg.server_status.lock().await;
        assert_eq!(server_status, PgServerStatus::Initialized);
    }

    pg.start_db().await?;
    {
        let server_status = *pg.server_status.lock().await;
        assert_eq!(server_status, PgServerStatus::Started);
    }

    pg.stop_db().await?;
    {
        let server_status = *pg.server_status.lock().await;
        assert_eq!(server_status, PgServerStatus::Stopped);
    }

    Ok(())
}

#[tokio::test]
#[serial]
async fn postgres_server_drop() -> Result<(), PgEmbedError> {
    let db_path = PathBuf::from("data_test/db");
    {
        let mut pg = common::setup(5432, db_path.clone(), false, None).await?;
        pg.start_db().await?;
        let file_exists = PgAccess::pg_version_file_exists(&db_path).await?;
        assert_eq!(true, file_exists);
    }
    let file_exists = PgAccess::pg_version_file_exists(&db_path).await?;
    assert_eq!(false, file_exists);
    Ok(())
}

#[tokio::test]
#[serial]
async fn postgres_server_multiple_concurrent() -> Result<(), PgEmbedError> {
    PgAccess::purge().await?;

    let tasks = vec![
        common::setup(5432, PathBuf::from("data_test/db1"), false, None),
        common::setup(5434, PathBuf::from("data_test/db3"), false, None),
    ];

    let wrap_with_mutex =
        |val: Result<PgEmbed, PgEmbedError>| val.map(|pg| Mutex::new(pg)).unwrap();

    let pgs: Vec<Mutex<PgEmbed>> = futures::future::join_all(tasks)
        .await
        .into_iter()
        .map(wrap_with_mutex)
        .collect();

    futures::stream::iter(&pgs)
        .for_each_concurrent(None, |pg| async move {
            let mut pg = pg.lock().await;
            let _ = pg.start_db().await;
            {
                let server_status = *pg.server_status.lock().await;
                assert_eq!(server_status, PgServerStatus::Started);
            }
        })
        .await;

    futures::stream::iter(&pgs)
        .for_each_concurrent(None, |pg| async move {
            let mut pg = pg.lock().await;
            let _ = pg.stop_db().await;
            {
                let server_status = *pg.server_status.lock().await;
                assert_eq!(server_status, PgServerStatus::Stopped);
            }
        })
        .await;

    Ok(())
}

#[tokio::test]
#[serial]
async fn postgres_server_persistent_true() -> Result<(), PgEmbedError> {
    let db_path = PathBuf::from("data_test/db");
    let mut database_dir = PathBuf::new();
    let mut pw_file_path = PathBuf::new();
    {
        let pg = common::setup(5432, db_path.clone(), true, None).await?;
        database_dir.clone_from(&pg.pg_access.database_dir);
        pw_file_path.clone_from(&pg.pg_access.pw_file_path);
        let file_exists = PgAccess::pg_version_file_exists(&db_path).await?;
        assert_eq!(true, file_exists);
    }
    let file_exists = PgAccess::pg_version_file_exists(&db_path).await?;
    assert_eq!(true, file_exists);

    PgAccess::clean_up(database_dir, pw_file_path).await?;

    let file_exists = PgAccess::pg_version_file_exists(&db_path).await?;
    assert_eq!(false, file_exists);

    Ok(())
}

#[tokio::test]
#[serial]
async fn postgres_server_persistent_false() -> Result<(), PgEmbedError> {
    let db_path = PathBuf::from("data_test/db");
    {
        let _pg = common::setup(5432, db_path.clone(), false, None).await?;
        let file_exists = PgAccess::pg_version_file_exists(&db_path).await?;
        assert_eq!(true, file_exists);
    }
    let file_exists = PgAccess::pg_version_file_exists(&db_path).await?;
    assert_eq!(false, file_exists);

    Ok(())
}

#[tokio::test]
#[serial]
async fn postgres_server_timeout() -> Result<(), PgEmbedError> {
    let database_dir = PathBuf::from("data_test/db");
    let _ = env_logger::Builder::from_env(Env::default().default_filter_or("info"))
        .is_test(true)
        .try_init();
    let pg_settings = PgSettings {
        database_dir,
        port: 5432,
        user: "postgres".to_string(),
        password: "password".to_string(),
        auth_method: PgAuthMethod::MD5,
        persistent: false,
        timeout: Some(Duration::from_secs(10)),
        migration_dir: None,
    };
    let fetch_settings = PgFetchSettings {
        version: PG_V15,
        ..Default::default()
    };
    let mut pg = PgEmbed::new(pg_settings, fetch_settings).await?;
    let _ = pg.setup().await;
    pg.pg_settings.timeout = Some(Duration::from_millis(10));
    let res = pg.start_db().await.err().map(|e| e.message).flatten();
    assert_eq!(Some("timed out".to_string()), res);

    Ok(())
}
