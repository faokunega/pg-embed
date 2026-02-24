<p align="center">
  <img src="assets/logo.jpg" alt="pg-embed logo" width="320">
</p>
<h1 align="center">pg-embed</h1>

<p align="center">
  <a href="https://crates.io/crates/pg-embed"><img src="https://img.shields.io/crates/v/pg-embed" alt="crates.io"></a>
  <a href="https://docs.rs/pg-embed"><img src="https://docs.rs/pg-embed/badge.svg" alt="docs.rs"></a>
  <a href="https://crates.io/crates/pg-embed"><img src="https://img.shields.io/crates/d/pg-embed" alt="downloads"></a>
  <img src="https://img.shields.io/badge/rustc-1.88%2B-orange" alt="MSRV 1.88">
  <a href="LICENSE"><img src="https://img.shields.io/crates/l/pg-embed" alt="license"></a>
</p>

<p align="center">
  Run a PostgreSQL server locally as part of a Rust application or test suite — no system installation required.
</p>

<p align="center">
  <img src="assets/banner.png" alt="pg-embed banner">
</p>

---

pg-embed downloads precompiled PostgreSQL binaries from [zonkyio/embedded-postgres-binaries](https://github.com/zonkyio/embedded-postgres-binaries), caches them on first use, and manages the full server lifecycle (`initdb` → `pg_ctl start` → `pg_ctl stop`). Built on [tokio](https://crates.io/crates/tokio).

## Contents

- [Quick start](#quick-start)
- [Extensions](#extensions)
- [Features](#features)
- [Platform support](#platform-support)
- [Binary cache](#binary-cache)
- [Documentation](#documentation)

---

## Quick start

```toml
[dependencies]
pg-embed = "1.0"
```

```rust,no_run
use std::path::PathBuf;
use std::time::Duration;

use pg_embed::pg_enums::PgAuthMethod;
use pg_embed::pg_errors::Result;
use pg_embed::pg_fetch::{PgFetchSettings, PG_V18};
use pg_embed::postgres::{PgEmbed, PgSettings};

#[tokio::main]
async fn main() -> Result<()> {
    let mut pg = PgEmbed::new(
        PgSettings {
            database_dir:  PathBuf::from("data/db"),
            port:          5432,
            user:          "postgres".to_string(),
            password:      "password".to_string(),
            auth_method:   PgAuthMethod::MD5,
            persistent:    false,
            timeout:       Some(Duration::from_secs(30)),
            migration_dir: None,
        },
        PgFetchSettings { version: PG_V18, ..Default::default() },
    ).await?;

    pg.setup().await?;    // download + unpack + initdb (cached after first run)
    pg.start_db().await?;

    pg.create_database("mydb").await?;

    // postgres://postgres:password@localhost:5432/mydb
    let uri = pg.full_db_uri("mydb");
    println!("Connect at: {uri}");

    pg.stop_db().await?;
    Ok(())
}
```

---

## Extensions

Third-party extensions (pgvector, PostGIS, etc.) are not included in the precompiled binaries. To install one, point `install_extension()` at a directory containing the pre-built files for your platform. Call it **after** `setup()` and **before** `start_db()`.

```rust,no_run
use std::path::Path;

// After pg.setup() and before pg.start_db():
pg.install_extension(Path::new("extensions/pgvector")).await?;
pg.start_db().await?;

// Then activate it inside a database:
// CREATE EXTENSION IF NOT EXISTS vector;
```

Files are routed by extension:

| File type               | Destination                           |
| :---------------------- | :------------------------------------ |
| `.so`, `.dylib`, `.dll` | `{cache}/lib/`                        |
| `.control`, `.sql`      | `{cache}/share/postgresql/extension/` |
| Anything else           | Skipped                               |

Pure-SQL extensions (no shared library) work the same way — simply omit the binary.

---

## Features

| Capability               | API                                                         | Feature flag       |
| :----------------------- | :---------------------------------------------------------- | :----------------- |
| 🔄 Server lifecycle       | `setup()`, `start_db()`, `stop_db()`                        | `rt_tokio`         |
| 🧩 Extension installation | `install_extension()`                                       | `rt_tokio`         |
| 🗄️ Database management    | `create_database()`, `drop_database()`, `database_exists()` | `rt_tokio_migrate` |
| 🚀 Migrations             | `migrate()`                                                 | `rt_tokio_migrate` |

The default feature is `rt_tokio_migrate` (includes sqlx). For a smaller build without sqlx:

```toml
pg-embed = { version = "1.0", default-features = false, features = ["rt_tokio"] }
```

Additional behaviours included in all builds:

- **Binary caching** — binaries are downloaded once per OS/arch/version and reused across runs.
- **Automatic shutdown** — `pg_ctl stop` is called on drop if the server is still running.
- **Concurrent safety** — a global lock prevents duplicate downloads when multiple instances initialise simultaneously.

---

## Platform support

| OS                                                                                                             | Architectures                                   |
| :------------------------------------------------------------------------------------------------------------- | :---------------------------------------------- |
| ![macOS](https://img.shields.io/badge/macOS-000000?style=flat&logo=apple&logoColor=white)                      | amd64, arm64v8 ¹                                |
| ![Linux](https://img.shields.io/badge/Linux-FCC624?style=flat&logo=linux&logoColor=black)                      | amd64, i386, arm32v6, arm32v7, arm64v8, ppc64le |
| ![Alpine Linux](https://img.shields.io/badge/Alpine_Linux-0D597F?style=flat&logo=alpine-linux&logoColor=white) | amd64, i386, arm32v6, arm32v7, arm64v8, ppc64le |
| ![Windows](https://img.shields.io/badge/Windows-0078D6?style=flat&logo=windows&logoColor=white)                | amd64, i386                                     |

Supported PostgreSQL versions: **10 – 18** (`PG_V10` … `PG_V18` constants).

¹ Apple Silicon binaries are available for PostgreSQL 14 and later only.

---

## Binary cache

Binaries are stored at an OS-specific location and reused on subsequent runs:

| OS      | Cache path                                          |
| :------ | :-------------------------------------------------- |
| macOS   | `~/Library/Caches/pg-embed/`                        |
| Linux   | `$XDG_CACHE_HOME/pg-embed/` or `~/.cache/pg-embed/` |
| Windows | `%LOCALAPPDATA%\pg-embed\`                          |

---

## Documentation

- [User Handbook](docs/user-handbook.md) — configuration reference, auth methods, migrations, FAQ
- [Technical Handbook](docs/technical-handbook.md) — architecture, data flow, module graph, internals
- [API reference on docs.rs](https://docs.rs/pg-embed)

---

## License

pg-embed is dual-licensed under [MIT](LICENSE) / [Apache 2.0](LICENSE).

---

## Credits

Precompiled binaries provided by [zonkyio/embedded-postgres-binaries](https://github.com/zonkyio/embedded-postgres-binaries), hosted on [Maven Central](https://mvnrepository.com/artifact/io.zonky.test.postgres/embedded-postgres-binaries-bom).
