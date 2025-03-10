use pg_embed::pg_enums::PgAuthMethod;
use pg_embed::pg_fetch::{PgFetchSettings, PG_V13};
use pg_embed::postgres::{PgEmbed, PgSettings};
use sqlx_tokio::postgres::PgPoolOptions;
use std::error::Error;
use std::path::PathBuf;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Postgresql settings
    let pg_settings = PgSettings {
        // Where to store the postgresql database
        database_dir: PathBuf::from("data/db"),
        port: 5432,
        user: "postgres".to_string(),
        password: "password".to_string(),
        // authentication method
        auth_method: PgAuthMethod::Plain,
        // If persistent is false clean up files and directories on drop, otherwise keep them
        persistent: false,
        // duration to wait before terminating process execution
        // pg_ctl start/stop and initdb timeout
        // if set to None the process will not be terminated
        timeout: Some(Duration::from_secs(15)),
        // If migration sql scripts need to be run, the directory containing those scripts can be
        // specified here with `Some(PathBuf(path_to_dir)), otherwise `None` to run no migrations.
        // To enable migrations view the **Usage** section for details
        migration_dir: None,
    };

    // Postgresql binaries download settings
    let fetch_settings = PgFetchSettings {
        version: PG_V13,
        ..Default::default()
    };

    // Create a new instance
    let mut pg = PgEmbed::new(pg_settings, fetch_settings).await?;

    // Download, unpack, create password file and database cluster
    pg.setup().await?;

    // start postgresql database
    pg.start_db().await?;

    pg.create_database("database_name").await?;

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&pg.full_db_uri("database_name"))
        .await?;

    let row: (i64,) = sqlx_tokio::query_as("SELECT $1")
        .bind(150_i64)
        .fetch_one(&pool)
        .await?;

    println!("{}", row.0);

    // stop postgresql database
    pg.stop_db().await?;

    Ok(())
}
