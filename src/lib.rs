//!
//! # pg-embed
//!
//! Run a Postgresql database locally on Linux, MacOS or Windows as part of another Rust application or test.
//!
//! # Usage
//!
//! A postgresql instance can be created using<br/>
//! [postgres::PgEmbed]::new([postgres::PgSettings], [fetch::FetchSettings]) <br/>
//!
//!
//! # Examples
//!
//! ```
//! use pg_embed::postgres::{PgEmbed, PgSettings};
//! use pg_embed::fetch;
//! use pg_embed::fetch::{OperationSystem, Architecture, FetchSettings, PG_V13};
//!
//! let pg_settings = PgSettings{
//!     executables_dir: "data/postgres".to_string(),
//!     database_dir: "data/db".to_string(),
//!     user: "postgres".to_string(),
//!     password: "password".to_string(),
//!     persistent: false
//! };
//! let fetch_settings = FetchSettings{
//!     host: "https://repo1.maven.org".to_string(),
//!     operating_system: OperationSystem::Darwin,
//!     architecture: Architecture::Amd64,
//!     version: PG_V13
//! };
//! let mut pg_emb = PgEmbed::new(pg_settings, fetch_settings);
//!
//! async {
//!     /// download postgresql
//!     pg_emb.aquire_postgres().await;
//!
//!     /// initialize postgresql database
//!     pg_emb.init_db().await;
//!
//!     /// start postgresql database
//!     pg_emb.start_db().await;
//!
//!     /// stop postgresql database
//!     pg_emb.stop_db().await;
//! }
//!
//!
//! ```
//!
pub mod fetch;
pub mod postgres;

