# pg-embed

[![Crates.io](https://img.shields.io/crates/v/pg-embed)](http://crates.io/crates/pg-embed)
[![Docs.rs](https://docs.rs/pg-embed/badge.svg)](https://docs.rs/pg-embed)
[![Crates.io](https://img.shields.io/crates/d/pg-embed)](http://crates.io/crates/pg-embed)
[![Crates.io](https://img.shields.io/crates/l/pg-embed)](https://github.com/faokunega/pg-embed/blob/master/LICENSE)

Run a Postgresql database locally on Linux, MacOS or Windows as part of another Rust application or test.

The currently supported async runtime for **pg-embed** is [tokio](https://crates.io/crates/tokio).

# Usage

- Add pg-embed to your Cargo.toml

  *Library without sqlx migration support*

     ```toml
     # Cargo.toml
     [dependencies]
     pg-embed = { version = "0.9", default-features = false, features = ["rt_tokio"] }
     ```

  *Library with sqlx migration support*

     ```toml
     # Cargo.toml
     [dependencies]
     pg-embed = "0.9"
     ```


# Examples

 ```rust
 use pg_embed::postgres::{PgSettings, PgEmbed};
use pg_embed::pg_fetch::{PgFetchSettings, PG_V17};
use pg_embed::pg_enums::PgAuthMethod;
use std::time::Duration;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), pg_embed::pg_errors::PgEmbedError> {
    /// Postgresql settings
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

    /// Postgresql binaries download settings
    let fetch_settings = PgFetchSettings {
        version: PG_V17,
        ..Default::default()
    };

    // Use an async block that returns `Result`
    // Create a new instance
    let mut pg = PgEmbed::new(pg_settings, fetch_settings).await?;

    // Download, unpack, create password file and database cluster
    pg.setup().await?;

    // start postgresql database
    pg.start_db().await?;

    // create a new database
    // to enable migrations view the [Usage] section for details
    pg.create_database("database_name").await?;

    // drop a database
    // to enable migrations view [Usage] for details
    pg.drop_database("database_name").await?;

    // get the base postgresql uri
    // `postgres://{username}:{password}@localhost:{port}`
    let pg_uri: &str = &pg.db_uri;

    // get a postgresql database uri
    // `postgres://{username}:{password}@localhost:{port}/{specified_database_name}`
    let pg_db_uri: String = pg.full_db_uri("database_name");

    // check database existence
    // to enable migrations view [Usage] for details
    pg.database_exists("database_name").await?;

    // run migration sql scripts
    // to enable migrations view [Usage] for details
    pg.migrate("database_name").await?;

    // stop postgresql database
    pg.stop_db().await?;

    // Return success
    println!("PostgreSQL setup completed successfully!");
    Ok(())
}
 ```
## Info

The downloaded postgresql binaries are cached in the following directories:

   - On Linux:

     `$XDG_CACHE_HOME/pg-embed`

     or

     `$HOME/.cache/pg-embed`
   - On Windows:

     `{FOLDERID_LocalAppData}/pg-embed`
   - On MacOS:

     `$HOME/Library/Caches/pg-embed`


## Recent Breaking Changes

pg-embed follows semantic versioning, so breaking changes should only happen upon major version bumps. The only
exception to this rule is breaking changes that happen due to implementation that was deemed to be a bug, security
concerns, or it can be reasonably proved to affect no code. For the full details,
see [CHANGELOG.md](https://github.com/faokunega/pg-embed/blob/master/CHANGELOG.md).

## License

pg-embed is licensed under the MIT license. Please read
the [LICENSE-MIT](https://github.com/faokunega/pg-embed/blob/master/LICENSE) file in this repository for more
information.

# Notes

Reliant on the great work being done
by [zonkyio/embedded-postgres-binaries](https://github.com/zonkyio/embedded-postgres-binaries) in order to fetch
precompiled binaries
from [Maven](https://mvnrepository.com/artifact/io.zonky.test.postgres/embedded-postgres-binaries-bom).

