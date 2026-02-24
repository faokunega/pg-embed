# pg-embed Technical Handbook

Internal reference covering architecture, data flow, global state, platform support, and error taxonomy.

---

## Module overview

```
src/
├── lib.rs               — public re-exports + compile_error! feature guard
├── pg_errors.rs         — Error enum (thiserror) + Result alias
├── pg_types.rs          — PgCommandSync type alias
├── pg_enums.rs          — PgAuthMethod, PgServerStatus, OperationSystem, Architecture, …
├── pg_fetch.rs          — HTTP download (reqwest) → raw JAR bytes
├── pg_unpack.rs         — JAR → XZ tarball → binary files on disk
├── pg_access.rs         — filesystem layout + ACQUIRED_PG_BINS global
├── pg_commands.rs       — builds AsyncCommandExecutor for initdb / pg_ctl
├── command_executor.rs  — generic async process runner with timeout
└── postgres.rs          — PgEmbed public API + PgSettings + Drop
```

### Dependency graph

```
postgres.rs
  ├── pg_access.rs
  │     └── pg_fetch.rs → (network)
  │     └── pg_unpack.rs → (disk)
  ├── pg_commands.rs
  │     └── command_executor.rs → (child processes)
  └── pg_enums.rs
        └── command_executor.rs (ProcessStatus trait)
```

All error types flow upward into `pg_errors::Error`; all async code runs on a tokio runtime.

---

## Data flow

### `PgEmbed::new()`

1. Accepts `PgSettings` and `PgFetchSettings`.
2. Constructs a `PgAccess` which computes:
   - **cache dir** (`{OS cache}/pg-embed/{os}/{arch}/{version}/`) — where binaries live.
   - **database dir** — user-supplied path for the cluster data files.
   - **password file path** — a `.pgpass`-style temp file written alongside the database dir.
3. Returns an uninitialised `PgEmbed` with `server_status = Uninitialized`.

### `pg.setup()`

```
PgEmbed::setup()
  └─ PgAccess::maybe_acquire_postgres()
       ├─ (if not cached) PgFetchSettings::fetch_postgres()    → Vec<u8>
       ├─ write zip to cache_dir
       ├─ pg_unpack::unpack_postgres(zip_path, cache_dir)
       │     ├─ tokio::task::spawn_blocking(...)
       │     ├─ ZipArchive::new(zip_file)
       │     ├─ find entry ending in ".txz" or ".xz"
       │     ├─ lzma_rs::xz_decompress(xz_bytes) → tar_bytes
       │     └─ Archive::new(tar_bytes).unpack(cache_dir)
       └─ mark ACQUIRED_PG_BINS[cache_dir] = Finished
  └─ write password file
  └─ (if no PG_VERSION file) run initdb
       └─ AsyncCommandExecutor { initdb, --auth, --username, --pwfile, … }
```

### `pg.install_extension(dir)`

```
PgEmbed::install_extension(extension_dir)
  └─ PgAccess::install_extension(extension_dir)
       ├─ locate share/postgresql/extension/ inside cache_dir
       │    (checks for existing dir; falls back to share/postgresql/extension)
       ├─ create lib/ and share/postgresql/extension/ if absent
       └─ for each file in extension_dir:
            ├─ .so / .dylib / .dll  → copy to {cache}/lib/
            ├─ .control / .sql      → copy to {cache}/share/postgresql/extension/
            └─ anything else        → skip
```

Must be called after `setup()` (cache dir exists) and before `start_db()` (PostgreSQL reads the extension directory at startup).

### `pg.start_db()`

```
PgEmbed::start_db()
  └─ pg_commands::pg_ctl_start(bin_dir, db_dir, port)
       └─ AsyncCommandExecutor::execute(timeout)
            ├─ tokio::process::Command::spawn()
            ├─ channel: stdout/stderr → log::info!
            └─ tokio::time::timeout(timeout, wait_for_exit)
  └─ server_status = Started
```

### `pg.stop_db()`

Mirror of `start_db`, calls `pg_ctl stop -w`. Also invoked synchronously from `Drop` via `stop_db_sync()` (uses `std::process::Command`).

### `Drop` implementation

```rust
impl Drop for PgEmbed {
    fn drop(&mut self) {
        // Synchronous — must not block the async executor
        self.stop_db_sync();          // std::process::Command
        if !self.pg_settings.persistent {
            // best-effort cleanup; errors are logged, not propagated
            let _ = block_on(PgAccess::clean(...));
        }
    }
}
```

**Constraint:** Because `Drop` is synchronous, cleanup that requires async (e.g. sqlx) cannot be done here.

---

## Global state

```rust
// src/pg_access.rs
static ACQUIRED_PG_BINS: LazyLock<Arc<Mutex<HashMap<PathBuf, PgAcquisitionStatus>>>> =
    LazyLock::new(|| Arc::new(Mutex::new(HashMap::with_capacity(5))));
```

**Purpose:** Prevents two concurrent `PgEmbed` instances from downloading or unpacking the same binary package simultaneously.

**Lifecycle:**
1. `maybe_acquire_postgres()` locks the mutex.
2. If the entry is `Undefined`, it sets it to `InProgress`, releases the lock, downloads + unpacks, then re-acquires and sets `Finished`.
3. If the entry is `InProgress`, the function polls (sleeps) until it becomes `Finished`.
4. If `Finished`, it skips straight to writing the password file.

**`purge()`** removes the entire `pg-embed` cache directory from disk and resets the map.

---

## Binary package format

Binaries are distributed as Maven JAR files (ZIP archives) from `repo1.maven.org`.
URL template:
```
{host}/maven2/io/zonky/test/postgres/
  embedded-postgres-binaries-{platform}/{version}/
  embedded-postgres-binaries-{platform}-{version}.jar
```

The JAR contains exactly one entry with a `.txz` extension (e.g. `postgres-darwin-arm_64.txz`).
That entry is an XZ-compressed tarball (`tar.xz`) containing the full PostgreSQL binary tree.

The unpacker:
1. Opens the JAR with the `zip` crate.
2. Finds the entry ending in `.txz` or `.xz`.
3. Decompresses XZ with `lzma-rs` (pure Rust).
4. Extracts the tar with the `tar` crate into the cache directory.

This is run inside `tokio::task::spawn_blocking` to avoid blocking the async executor.

---

## Filesystem layout

```
{cache}/pg-embed/{os}/{arch}/{version}/
  ├── bin/
  │    ├── initdb
  │    ├── pg_ctl
  │    └── postgres (and other PG tools)
  ├── lib/                             ← .so/.dylib/.dll from install_extension()
  ├── share/postgresql/extension/      ← .control/.sql from install_extension()
  └── {version}.zip   ← downloaded JAR (kept as cache marker)

{database_dir}/
  ├── PG_VERSION        ← created by initdb; used as existence check
  ├── pg_hba.conf
  └── … (standard cluster data files)

{database_dir}/../.pgpass   ← password file written by setup()
```

Cache base path (OS-specific, from `dirs` crate):

| OS      | Path |
|---------|------|
| macOS   | `$HOME/Library/Caches/pg-embed` |
| Linux   | `$XDG_CACHE_HOME/pg-embed` or `$HOME/.cache/pg-embed` |
| Windows | `{FOLDERID_LocalAppData}\pg-embed` |

---

## Platform support matrix

| OS            | `OperationSystem` variant | `Display` string |
|---------------|--------------------------|-----------------|
| macOS         | `Darwin`                 | `darwin`        |
| Linux (glibc) | `Linux`                  | `linux`         |
| Alpine Linux  | `AlpineLinux`            | `linux`         |
| Windows       | `Windows`                | `windows`       |

| CPU           | `Architecture` variant | `Display` string |
|---------------|------------------------|-----------------|
| x86-64        | `Amd64`                | `amd64`         |
| 32-bit x86    | `I386`                 | `i386`          |
| ARMv6 32-bit  | `Arm32v6`              | `arm32v6`       |
| ARMv7 32-bit  | `Arm32v7`              | `arm32v7`       |
| AArch64       | `Arm64v8`              | `arm64v8`       |
| POWER LE 64   | `Ppc64le`              | `ppc64le`       |

For Alpine Linux the Maven classifier appends `-alpine` to the architecture:
`linux-amd64-alpine`, `linux-arm64v8-alpine`, etc.

`OperationSystem::default()` and `Architecture::default()` are set at compile time via `#[cfg(target_os)]` / `#[cfg(target_arch)]`.

---

## Feature flags

| Feature              | Enables                    | Gates                                                      |
|----------------------|----------------------------|------------------------------------------------------------|
| `rt_tokio`           | tokio + reqwest            | fetch, unpack, init, start/stop, `install_extension`       |
| `rt_tokio_migrate`   | + sqlx                     | everything above + `create_database`, `drop_database`, `database_exists`, `migrate` |

At least one feature is required; `lib.rs` emits `compile_error!` otherwise.
The default features are `rt_tokio_migrate`.

sqlx-dependent code is guarded with `#[cfg(feature = "rt_tokio_migrate")]`.

---

## `command_executor.rs` internals

`AsyncCommandExecutor` implements the `AsyncCommand` trait using AFIT (async fn in traits, stable since Rust 1.75).

```
execute(timeout)
  ├─ tokio::process::Command::spawn()
  ├─ mpsc channel for stdout + stderr lines
  ├─ tokio::spawn task: BufReader::lines() → channel.send()
  ├─ tokio::spawn task: read channel → log::info!
  └─ tokio::time::timeout(timeout, wait())
       ├─ Ok(Ok(status)) → update server_status
       ├─ Ok(Err(_))     → Error::PgProcessError
       └─ Err(Elapsed)   → Error::PgTimedOutError
```

`server_status` is a `Arc<Mutex<PgServerStatus>>` shared between the executor and the `PgEmbed` struct, updated at entry (`Initializing` / `Starting` / `Stopping`) and exit (`Initialized` / `Started` / `Stopped` or `Failure`).

---

## Error taxonomy

| Variant              | When raised |
|----------------------|-------------|
| `InvalidPgUrl`       | Cache directory unavailable or unsupported platform |
| `InvalidPgPackage`   | Downloaded ZIP cannot be opened or has no `.txz`/`.xz` entry |
| `WriteFileError`     | Password file or zip file write fails |
| `ReadFileError`      | File read or existence check fails |
| `DirCreationError`   | `fs::create_dir_all` fails |
| `UnpackFailure`      | XZ decompress or tar extract fails |
| `PgStartFailure`     | `pg_ctl start` exits non-zero |
| `PgStopFailure`      | `pg_ctl stop` exits non-zero |
| `PgInitFailure`      | `initdb` exits non-zero |
| `PgCleanUpFailure`   | Removal of database dir or password file fails |
| `PgPurgeFailure`     | Removal of cache directory fails |
| `PgBufferReadError`  | BufReader line read fails inside I/O task |
| `PgLockError`        | Mutex acquire fails |
| `PgProcessError`     | `child.wait()` or spawn fails |
| `PgTimedOutError`    | `tokio::time::timeout` elapsed |
| `PgTaskJoinError`    | `spawn_blocking` task panicked |
| `PgError`            | Internal context wrapper (message + context string) |
| `DownloadFailure`    | `reqwest::get` fails |
| `ConversionFailure`  | `.bytes().await` fails on HTTP response |
| `SendFailure`        | MPSC channel send fails (receiver dropped) |
| `SqlQueryError`      | sqlx connection or query fails (`rt_tokio_migrate`) |
| `MigrationError`     | sqlx migrator fails (`rt_tokio_migrate`) |

---

## MSRV and dependency notes

- **MSRV:** Rust 1.88 — set by `zip` 8.x (1.88); Rust edition 2024 requires 1.85; `std::sync::LazyLock` requires 1.80
- **`lzma-rs`:** pure-Rust XZ decompression; replaces former C-based `xz2`
- **`std::sync::LazyLock`:** replaces former `lazy_static` crate for `ACQUIRED_PG_BINS`
- **AFIT:** replaces former `async-trait` crate in `AsyncCommand`
- **`zip` 8.x:** binding MSRV constraint at 1.88
- **`reqwest` 0.13:** TLS backend is `rustls` (no OpenSSL dependency)

---

## Testing

Integration tests are split into thematic files:

| File | Requires | Content |
|:---|:---|:---|
| `tests/lifecycle.rs` | `rt_tokio` + `rt_tokio_migrate` | start/stop, drop, timeout, persistence, concurrent |
| `tests/auth.rs` | `rt_tokio` + `rt_tokio_migrate` | authentication methods |
| `tests/database.rs` | `rt_tokio_migrate` | create/drop/exists, URI format |
| `tests/migration.rs` | `rt_tokio_migrate` | sqlx migrations |
| `tests/extension.rs` | `rt_tokio_migrate` | extension install and use |

`lifecycle.rs` and `auth.rs` are each registered twice in `Cargo.toml` (once per feature flag) so the same source is compiled under both `rt_tokio` and `rt_tokio_migrate`.

`#[file_serial(pg_port_5432)]` is used on tests that start a server to serialise across process boundaries (file locks work between separate test binaries).

Test isolation uses `tempfile::TempDir` via the `tests/common::setup_with_tempdir` helper. Return type is `(TempDir, PgEmbed)` — `TempDir` first so it is dropped _last_, after `PgEmbed` has stopped the server and removed the database directory.
