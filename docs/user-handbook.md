# pg-embed User Handbook

Practical guide to embedding a PostgreSQL server in your Rust application or test suite.

---

## Installation

Add to `Cargo.toml`:

```toml
[dependencies]
pg-embed = "1.0"
```

The default feature set (`rt_tokio_migrate`) includes tokio, reqwest, and sqlx.
For a smaller build without sqlx/migrations:

```toml
[dependencies]
pg-embed = { version = "1.0", default-features = false, features = ["rt_tokio"] }
```

At least one feature must be enabled.

---

## Quick start

```rust,no_run
use std::time::Duration;
use pg_embed::pg_enums::PgAuthMethod;
use pg_embed::pg_errors::Result;
use pg_embed::pg_fetch::{PgFetchSettings, PG_V17};
use pg_embed::postgres::{PgEmbed, PgSettings};

#[tokio::main]
async fn main() -> Result<()> {
    let pg_settings = PgSettings {
        database_dir:  std::path::PathBuf::from("data/db"),
        port:          5432,
        user:          "postgres".to_string(),
        password:      "password".to_string(),
        auth_method:   PgAuthMethod::MD5,
        persistent:    false,
        timeout:       Some(Duration::from_secs(30)),
        migration_dir: None,
    };

    let fetch_settings = PgFetchSettings {
        version: PG_V17,
        ..Default::default()
    };

    let mut pg = PgEmbed::new(pg_settings, fetch_settings).await?;
    pg.setup().await?;   // download + unpack + initdb (cached after first run)
    pg.start_db().await?;

    // connection string: "postgres://postgres:password@localhost:5432/postgres"
    println!("{}", pg.full_db_uri("postgres"));

    pg.stop_db().await?;
    Ok(())
}
```

`setup()` downloads the binary package on first use and caches it.  Subsequent runs skip the download.

---

## `PgSettings` reference

| Field          | Type                   | Required | Description |
|----------------|------------------------|----------|-------------|
| `database_dir` | `PathBuf`              | yes      | Directory for the PostgreSQL cluster data files. Created automatically. |
| `port`         | `u16`                  | yes      | TCP port the server listens on. |
| `user`         | `String`               | yes      | Superuser username (passed to `initdb`). |
| `password`     | `String`               | yes      | Superuser password (written to a temp file, passed to `initdb`). |
| `auth_method`  | `PgAuthMethod`         | yes      | Authentication method for `pg_hba.conf`. |
| `persistent`   | `bool`                 | yes      | If `false`, the cluster is deleted when `PgEmbed` is dropped. |
| `timeout`      | `Option<Duration>`     | yes      | Timeout for `initdb`, `pg_ctl start`, and `pg_ctl stop`. `None` = no timeout. |
| `migration_dir`| `Option<PathBuf>`      | yes      | Directory of `.sql` migration files. `None` = no migrations. |

---

## `PgFetchSettings` reference

| Field              | Type              | Default                    | Description |
|--------------------|-------------------|----------------------------|-------------|
| `host`             | `String`          | `https://repo1.maven.org`  | Maven repository base URL. Override to use a local mirror. |
| `operating_system` | `OperationSystem` | detected at compile time   | Target OS. |
| `architecture`     | `Architecture`    | detected at compile time   | Target CPU architecture. |
| `version`          | `PostgresVersion` | `PG_V17`                   | PostgreSQL version to download. Prefer an explicit constant. |

Available version constants: `PG_V10`, `PG_V11`, `PG_V12`, `PG_V13`, `PG_V14`, `PG_V15`, `PG_V16`, `PG_V17`, `PG_V18`.

---

## Authentication methods

| `PgAuthMethod` | `pg_hba.conf` value | Notes |
|----------------|---------------------|-------|
| `Plain`        | `password`          | Plaintext — for development only |
| `MD5`          | `md5`               | MD5-hashed password |
| `ScramSha256`  | `scram-sha-256`     | Recommended for PostgreSQL ≥ 11 |

---

## Platform support

Binaries are provided by [zonkyio/embedded-postgres-binaries](https://github.com/zonkyio/embedded-postgres-binaries).

| OS             | Architectures supported |
|----------------|------------------------|
| macOS          | amd64, arm64v8         |
| Linux (glibc)  | amd64, i386, arm32v6, arm32v7, arm64v8, ppc64le |
| Alpine Linux   | amd64, i386, arm32v6, arm32v7, arm64v8, ppc64le |
| Windows        | amd64, i386            |

The `OperationSystem` and `Architecture` defaults are detected at compile time.  Override via `PgFetchSettings` to cross-target.

---

## Connection strings

After `start_db()`, use:

```rust,no_run
// Base URI: postgres://{user}:{password}@localhost:{port}/{db}
let uri = pg.full_db_uri("mydb");
// e.g. "postgres://postgres:password@localhost:5432/mydb"
```

For the default `postgres` database:

```rust,no_run
let uri = pg.full_db_uri("postgres");
```

---

## Database operations (rt_tokio_migrate only)

These methods require the `rt_tokio_migrate` feature (the default).

```rust,no_run
pg.create_database("mydb").await?;
assert!(pg.database_exists("mydb").await?);
pg.drop_database("mydb").await?;
```

---

## Installing extensions

Third-party extensions (pgvector, PostGIS, etc.) are not included in the precompiled binaries. To use one, obtain the pre-built files for your platform and call `install_extension()` **after** `setup()` and **before** `start_db()`.

```rust,no_run
use std::path::Path;

pg.setup().await?;
pg.install_extension(Path::new("extensions/pgvector")).await?;
pg.start_db().await?;

// Then inside a database:
// CREATE EXTENSION IF NOT EXISTS vector;
```

`install_extension()` copies files from the given directory into the binary cache:

| File extension | Destination |
|:---|:---|
| `.so`, `.dylib`, `.dll` | `{cache}/lib/` |
| `.control`, `.sql` | `{cache}/share/postgresql/extension/` |
| Anything else | Skipped |

Pure-SQL extensions (no shared library) work identically — simply omit the binary file.

---

## Running migrations

Place numbered `.sql` files in a directory:

```
migrations/
  01_create_users.sql
  02_add_email.sql
```

Configure `PgSettings`:

```rust,no_run
PgSettings {
    migration_dir: Some(PathBuf::from("migrations")),
    // …
}
```

Then after starting and creating the database:

```rust,no_run
pg.create_database("mydb").await?;
pg.migrate("mydb").await?;
```

Migrations are applied in filename order using the sqlx migrator.

---

## Persistent vs. ephemeral clusters

| `persistent` | Behaviour on `Drop` |
|-------------|---------------------|
| `false`     | `stop_db_sync()` is called, then the database dir and password file are deleted. |
| `true`      | `stop_db_sync()` is called, data files are left on disk. |

For persistent clusters, call `PgAccess::clean_up(database_dir, pw_file_path)` to clean up manually.

---

## Binary cache

Downloaded binaries are cached at:

| OS      | Cache location |
|---------|----------------|
| macOS   | `~/Library/Caches/pg-embed/{os}/{arch}/{version}/` |
| Linux   | `~/.cache/pg-embed/{os}/{arch}/{version}/` |
| Windows | `%LOCALAPPDATA%\pg-embed\{os}\{arch}\{version}\` |

To clear the cache from code:

```rust,no_run
PgAccess::purge().await?;
```

---

## Multiple simultaneous instances

Run multiple servers on different ports:

```rust,no_run
let mut pg1 = PgEmbed::new(settings_on_5432, fetch_settings.clone()).await?;
let mut pg2 = PgEmbed::new(settings_on_5433, fetch_settings.clone()).await?;

pg1.setup().await?;
pg2.setup().await?;

pg1.start_db().await?;
pg2.start_db().await?;
```

A global lock (`ACQUIRED_PG_BINS`) ensures the binary package is only downloaded once even if both instances initialise concurrently.

---

## Using a local Maven mirror

Override `host` to point at a local artifact proxy:

```rust,no_run
let fetch_settings = PgFetchSettings {
    host: "https://my-artifactory.internal".to_string(),
    version: PG_V17,
    ..Default::default()
};
```

---

## Logging

pg-embed uses the `log` crate. Enable output with any compatible backend:

```rust,no_run
env_logger::Builder::from_env(
    env_logger::Env::default().default_filter_or("info")
).init();
```

For detailed output including `initdb` / `pg_ctl` stdout lines, use `RUST_LOG=debug`.

---

## FAQ

**Q: The first test run is slow.**
A: pg-embed downloads the binary package (~20–60 MB depending on OS/arch) on first use and caches it. Subsequent runs are fast.

**Q: Tests fail with "port already in use".**
A: Use `serial_test` to prevent concurrent tests from binding the same port:

```rust,no_run
use serial_test::file_serial;

#[tokio::test]
#[file_serial(pg_port_5432)]
async fn my_test() { … }
```

Use `#[file_serial]` (not `#[serial]`) if you run multiple test binaries, as file locks work across processes.

**Q: The data directory is not cleaned up after tests.**
A: Use `persistent: false` and ensure the `PgEmbed` value is dropped before the test ends. If you store the database dir in a `tempfile::TempDir`, declare the `TempDir` _before_ `PgEmbed` so that `PgEmbed` drops first.

**Q: How do I choose a timeout?**
A: `initdb` typically takes 1–5 seconds on a warm machine; `pg_ctl start` is similar. 10–30 seconds is safe for most CI environments. Set `timeout: None` to disable the timeout entirely.

**Q: `ScramSha256` fails on PostgreSQL 10.**
A: SCRAM-SHA-256 was introduced in PostgreSQL 10 but some client libraries only support it from PG 11. Use `PgAuthMethod::MD5` for maximum compatibility.

**Q: Can I use pg-embed without internet access?**
A: Yes. Set `host` to a local mirror URL, or pre-populate the cache directory at the path described above with the appropriate `.jar` / unpacked binaries.
