use serial_test::file_serial;
use tempfile::TempDir;

use pg_embed::pg_errors::{Error, Result};
use sqlx::{Connection, PgConnection};

#[path = "common.rs"]
mod common;

/// Verify that install_extension() stages files that PostgreSQL can actually load.
///
/// A minimal pure-SQL extension (no shared library needed) is written to a
/// temp dir, installed into the binary cache, and then activated with
/// `CREATE EXTENSION`.  The extension's function is called to confirm it is
/// fully operational.
#[tokio::test]
#[file_serial(pg_port_5432)]
async fn install_and_use() -> Result<()> {
    let (_dir, mut pg) = common::setup_with_tempdir(5432, false, None).await?;

    let ext_dir = TempDir::new().map_err(|e| Error::DirCreationError(e.to_string()))?;
    std::fs::write(
        ext_dir.path().join("pg_embed_test.control"),
        "comment = 'pg-embed integration test extension'\n\
         default_version = '1.0'\n\
         relocatable = true\n",
    )
    .map_err(|e| Error::WriteFileError(e.to_string()))?;
    std::fs::write(
        ext_dir.path().join("pg_embed_test--1.0.sql"),
        "CREATE FUNCTION pg_embed_test_hello() \
         RETURNS text LANGUAGE sql AS $$ SELECT 'hello from pg_embed_test'::text $$;\n",
    )
    .map_err(|e| Error::WriteFileError(e.to_string()))?;

    pg.install_extension(ext_dir.path()).await?;
    pg.start_db().await?;
    pg.create_database("exttest").await?;

    let mut conn = PgConnection::connect(&pg.full_db_uri("exttest"))
        .await
        .map_err(|e| Error::SqlQueryError(e.to_string()))?;

    sqlx::query("CREATE EXTENSION pg_embed_test")
        .execute(&mut conn)
        .await
        .map_err(|e| Error::SqlQueryError(e.to_string()))?;

    let greeting: String = sqlx::query_scalar("SELECT pg_embed_test_hello()")
        .fetch_one(&mut conn)
        .await
        .map_err(|e| Error::SqlQueryError(e.to_string()))?;

    assert_eq!(greeting, "hello from pg_embed_test");
    Ok(())
}
