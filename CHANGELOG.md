# v1.0.0
___
### Breaking Changes
- **MSRV bumped to 1.88** (was 1.80). Driven by `zip` 8.x (1.88) and Rust edition 2024 (1.85).
- **Rust edition 2024** — `edition = "2024"` in `Cargo.toml`.
- `async-trait` dependency removed — `AsyncCommand` trait now uses async fn in traits (AFIT, stable since Rust 1.75)
- `xz2` (C bindings) replaced by `lzma-rs` (pure Rust) for XZ decompression
- `lazy_static` removed — replaced by `std::sync::LazyLock`
- `bytes` crate removed as a direct dependency
- `futures` moved to dev-dependencies only

### Features
- **Extension installation** — new `PgEmbed::install_extension(dir)` (and `PgAccess::install_extension`) copies pre-built extension files into the binary cache between `setup()` and `start_db()`. Files are routed by extension: `.so`/`.dylib`/`.dll` → `lib/`; `.control`/`.sql` → `share/postgresql/extension/`. The target directory is discovered at runtime so the correct PostgreSQL share layout is used regardless of platform.
- **PostgreSQL 18 support** — added `PG_V18` constant; default version is now PG 18
- **Streaming download** — binaries are now streamed directly to disk instead of being buffered in memory (eliminates 100–200 MB peak RAM usage during setup)
- **Clear error for unsupported platforms** — attempting to download PG 10–13 on Apple Silicon now returns a descriptive `DownloadFailure` error instead of silently receiving corrupt data
- **`PgAccess::pg_version_file_exists`** — new public helper to check whether a cluster directory has been initialised

### Fixes
- `Drop` impl now logs errors from `pg_ctl stop` and cleanup instead of discarding them silently
- Dead binding in `setup()` removed; `initdb` errors now propagate correctly
- Process output channel closure in `command_executor` is now logged as a warning
- `pg_access`: path construction uses `PathBuf::join` instead of `clone().push()`, removing unnecessary allocations

### Tests
- New end-to-end extension test (`tests/extension.rs`) installs a pure-SQL extension, starts the server, runs `CREATE EXTENSION`, and asserts the extension function returns the expected value.
- Integration tests reorganised into thematic files: `lifecycle.rs`, `auth.rs`, `database.rs`, `migration.rs`, `extension.rs`.

### Dependencies updated
- `reqwest` 0.13, `tokio` 1.x (latest), `zip` 8.x, `thiserror` 2.x, `tempfile` 3.x, `sqlx` 0.8

### Documentation
- New **User Handbook** (`docs/user-handbook.md`) — configuration reference, auth methods, migrations, FAQ
- New **Technical Handbook** (`docs/technical-handbook.md`) — architecture, data flow, module graph, error taxonomy
- All public API items documented with `///` doc comments
- New **Extensions** section in `README.md` with a full pgvector-flavoured code example and file-routing table.

# v0.9.0
___
- Updated libraries
- Updated latest postgresql versions
- Fixed doc example code
- adjusted feature flags

# v0.6.5
___
### Fix
The errors introduced in v0.6.2 - v0.6.4 got fixed.
Big thanks to nicoulaj for his contribution

# v0.6.2 - v0.6.4
___
### Important notes
Due to error fixing for the Windows OS console some errors where introduced on other platforms.
Please use a newer version to prevent getting those errors.

# v0.6.0
___
### Feature
- Timeout can now be disabled through setting PgSettings{.., timeout: None}

### Breaking Changes
- PgSettings timeout attribute has been change to Option<Duration> (description above)

# v0.5.4
___
### Restructuring
- Extracted command execution

# v0.5.3
___
### Fix
- Fixed: Concurrent PgEmbed instances trying to acquire pg resources simultaneously

# v0.5.2
___
### Fix
- Password was created at wrong destination
- stopping db on drop fix

# v0.5.1
___
### Fix
- **PgEmbed**'s ***stop_db()*** did not execute on drop
- Multiple concurrent **PgEmbed** instances tried each to download the same resources when being setup

# v0.5.0
___
### Feature
> - Caching postgresql binaries
>    
>   Removed **executables_dir** attribute from **PgSettings**
> 
>   The downloaded postgresql binaries are now cached in the following directories:
>   
>   - On Linux:
>     
>     **$XDG_CACHE_HOME/pg-embed**
> 
>     or 
> 
>     **$HOME/.cache/pg-embed**
>   - On Windows: 
>     
>     **{FOLDERID_LocalAppData}/pg-embed**
>   - On MacOS:
> 
>     **$HOME/Library/Caches/pg-embed**
> 
>   Binaries download only happens if cached binaries are not found
> - Cleaner logging
>   
>   Logging is now done with the **log** crate. 
>   
>   In order to produce log output a logger implementation compatible with the facade has to be used.
>   
>   See https://crates.io/crates/log for detailed info
> 
>
### Breaking changes
**PgSettings** ***executables_dir*** attribute has been removed (*described above*).

### Thanks
❤️ - Big thanks to **nicoulaj** for his contribution

# v0.4.3
___
- migrator fix

# v0.4.2
___
- updated documentation

# v0.4.1
___
- updated documentation

# v0.4.0
___
### Fix
 - changed file path vars from String to PathBuf
 - password authentication

### Feature
> - added authentication methods to **PgSettings**
>   
>   Setting the **auth_method** property of **PgSettings**
>   to one of the following values will determine the authentication
>   method:
> 
>   - **PgAuthMethod::Plain**
>       
>       Plain-Text password
>   - **PgAuthMethod::Md5**
>       
>       Md5 password hash
> 
>   - **PgAuthMethod::ScramSha256**
> 
>       Sha256 password hash
>
> 

### Breaking changes
**PgSettings** has a new property called **auth_method** (*described above*).

This property has to be set.

# v0.3.2
___
### Fix
    - documentation updates

# v0.3.0
___
### Feature
> - added cargo features
> 
>   **rt_tokio** (*build with tokio async runtime and without sqlx db migration support*)
> 
>   **rt_tokio_migrate** (*build with tokio async runtime and sqlx db migration support*)

# v0.2.3
___
### Dependencies
    - added sqlx

### Fix
    - added start timeout

### Feature
    - added PgEmbed::create_database(name)

# v0.2.2
___

### Features
- added port setting to PgSettings

# v0.2.0
___

- switched from async-std to tokio
- switched from surf to reqwest

