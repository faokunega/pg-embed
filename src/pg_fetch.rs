//! Download PostgreSQL binaries from Maven Central.
//!
//! The [`PgFetchSettings`] struct describes *which* binary to fetch (OS,
//! architecture, version) and exposes [`PgFetchSettings::fetch_postgres`] to
//! perform the actual HTTP download.  The downloaded bytes are a JAR file
//! (ZIP) that is later unpacked by [`crate::pg_unpack`].

use std::path::Path;

use tokio::io::AsyncWriteExt;

use crate::pg_enums::{Architecture, OperationSystem};
use crate::pg_errors::Error;
use crate::pg_errors::Result;

/// A PostgreSQL version string in `MAJOR.MINOR.PATCH` form.
///
/// Use one of the provided constants ([`PG_V17`], [`PG_V16`], …) rather than
/// constructing this type directly.
#[derive(Debug, Copy, Clone)]
pub struct PostgresVersion(pub &'static str);


/// PostgreSQL 18.2.0 binaries.
pub const PG_V18: PostgresVersion = PostgresVersion("18.2.0");
/// PostgreSQL 17.8.0 binaries.
pub const PG_V17: PostgresVersion = PostgresVersion("17.8.0");
/// PostgreSQL 16.12.0 binaries.
pub const PG_V16: PostgresVersion = PostgresVersion("16.12.0");
/// PostgreSQL 15.16.0 binaries.
pub const PG_V15: PostgresVersion = PostgresVersion("15.16.0");
/// PostgreSQL 14.21.0 binaries.
pub const PG_V14: PostgresVersion = PostgresVersion("14.21.0");
/// PostgreSQL 13.23.0 binaries.
pub const PG_V13: PostgresVersion = PostgresVersion("13.23.0");
/// PostgreSQL 12.22.0 binaries.
pub const PG_V12: PostgresVersion = PostgresVersion("12.22.0");
/// PostgreSQL 11.22.1 binaries.
pub const PG_V11: PostgresVersion = PostgresVersion("11.22.1");
/// PostgreSQL 10.23.0 binaries.
pub const PG_V10: PostgresVersion = PostgresVersion("10.23.0");

/// Settings that determine which PostgreSQL binary package to download.
///
/// Construct with [`Default::default`] and override individual fields as
/// needed:
///
/// ```rust
/// use pg_embed::pg_fetch::{PgFetchSettings, PG_V17};
///
/// let settings = PgFetchSettings {
///     version: PG_V17,
///     ..Default::default()
/// };
/// ```
///
/// The default target OS and architecture are detected at compile time via
/// `#[cfg(target_os)]` / `#[cfg(target_arch)]`.
#[derive(Debug, Clone)]
pub struct PgFetchSettings {
    /// Base URL of the Maven repository hosting the binaries.
    ///
    /// Defaults to `https://repo1.maven.org`.  Override to point at a local
    /// mirror or artifact proxy.
    pub host: String,
    /// Target operating system.  Determines the package classifier used in the
    /// Maven artifact name.
    pub operating_system: OperationSystem,
    /// Target CPU architecture.  Combined with [`Self::operating_system`] to
    /// form the Maven classifier.
    pub architecture: Architecture,
    /// PostgreSQL version to download.  Use one of the `PG_Vxx` constants.
    pub version: PostgresVersion,
}

impl Default for PgFetchSettings {
    fn default() -> Self {
        PgFetchSettings {
            host: "https://repo1.maven.org".to_string(),
            operating_system: OperationSystem::default(),
            architecture: Architecture::default(),
            version: PG_V18,
        }
    }
}

impl PgFetchSettings {
    /// Returns the Maven classifier string for this OS/architecture combination.
    ///
    /// The classifier is the middle segment of the artifact name, e.g.
    /// `linux-amd64` or `darwin-amd64`.  For Alpine Linux the architecture
    /// gets an `-alpine` suffix instead of a separate OS segment.
    ///
    /// # Returns
    ///
    /// A `String` of the form `{os}-{arch}` (or `{os}-{arch}-alpine` for
    /// [`OperationSystem::AlpineLinux`]).
    pub fn platform(&self) -> String {
        let os = self.operating_system.to_string();
        let arch = if self.operating_system == OperationSystem::AlpineLinux {
            format!("{}-alpine", self.architecture)
        } else {
            self.architecture.to_string()
        };
        format!("{}-{}", os, arch)
    }

    /// Initiates an HTTP GET for the Maven artifact and checks the response status.
    ///
    /// Constructs the full artifact URL from [`Self::host`], [`Self::platform`],
    /// and [`Self::version`] and issues the request.  The caller streams the
    /// response body.
    ///
    /// # Errors
    ///
    /// Returns [`Error::DownloadFailure`] if the request fails or the server
    /// returns a non-2xx status.
    async fn start_download(&self) -> Result<reqwest::Response> {
        let platform = self.platform();
        let version = self.version.0;
        let download_url = format!(
            "{}/maven2/io/zonky/test/postgres/embedded-postgres-binaries-{}/{}/embedded-postgres-binaries-{}-{}.jar",
            &self.host,
            &platform,
            version,
            &platform,
            version
        );

        let response = reqwest::get(download_url)
            .await
            .map_err(|e| Error::DownloadFailure(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            return Err(Error::DownloadFailure(format!(
                "HTTP {status} fetching PostgreSQL {version} for platform '{platform}'. \
                 This version may not be available for the current OS/architecture. \
                 Note: darwin-arm64v8 (Apple Silicon) only has binaries for PG 14 and newer.",
            )));
        }

        Ok(response)
    }

    /// Downloads the PostgreSQL binaries JAR from Maven Central.
    ///
    /// Constructs the full artifact URL from [`Self::host`], [`Self::platform`],
    /// and [`Self::version`], performs an HTTP GET, and returns the raw bytes of
    /// the JAR file.  The caller is responsible for persisting and unpacking the
    /// data (see [`crate::pg_unpack::unpack_postgres`]).
    ///
    /// Prefer [`Self::fetch_postgres_to_file`] when the bytes will be written
    /// to disk — it streams directly without buffering the entire archive in
    /// memory.
    ///
    /// # Returns
    ///
    /// The raw bytes of the downloaded JAR on success.
    ///
    /// # Errors
    ///
    /// Returns [`Error::DownloadFailure`] if the HTTP request fails or the
    /// server returns a non-2xx status (e.g. 404 when the requested
    /// PostgreSQL version is not available for the current platform).
    /// Returns [`Error::ConversionFailure`] if reading the response body fails.
    pub async fn fetch_postgres(&self) -> Result<Vec<u8>> {
        let response = self.start_download().await?;
        let content = response
            .bytes()
            .await
            .map_err(|e| Error::ConversionFailure(e.to_string()))?;

        log::debug!("Downloaded {} bytes", content.len());
        log::trace!(
            "First 1024 bytes: {:?}",
            &String::from_utf8_lossy(&content[..content.len().min(1024)])
        );

        Ok(content.to_vec())
    }

    /// Downloads the PostgreSQL binaries JAR and streams it directly to `zip_path`.
    ///
    /// Unlike [`Self::fetch_postgres`], this method never loads the full archive
    /// into memory — each HTTP chunk is written to the file as it arrives.
    /// Use this method when you intend to write the JAR to disk (as
    /// [`crate::pg_access::PgAccess`] does), since it avoids a 100–200 MB
    /// in-memory buffer.
    ///
    /// # Arguments
    ///
    /// * `zip_path` — Destination file path for the downloaded JAR.
    ///
    /// # Errors
    ///
    /// Returns [`Error::DownloadFailure`] if the HTTP request fails or the
    /// server returns a non-2xx status.
    /// Returns [`Error::WriteFileError`] if the file cannot be created or a
    /// chunk cannot be written.
    /// Returns [`Error::ConversionFailure`] if reading a response chunk fails.
    pub(crate) async fn fetch_postgres_to_file(&self, zip_path: &Path) -> Result<()> {
        let mut response = self.start_download().await?;
        let mut file = tokio::fs::File::create(zip_path)
            .await
            .map_err(|e| Error::WriteFileError(e.to_string()))?;
        let mut total = 0u64;
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|e| Error::ConversionFailure(e.to_string()))?
        {
            file.write_all(&chunk)
                .await
                .map_err(|e| Error::WriteFileError(e.to_string()))?;
            total += chunk.len() as u64;
        }
        file.sync_data()
            .await
            .map_err(|e| Error::WriteFileError(e.to_string()))?;
        log::debug!("Downloaded and wrote {} bytes to disk", total);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

#[tokio::test]
    async fn fetch_postgres() -> Result<()> {
        let pg_settings = PgFetchSettings::default();
        let content = pg_settings.fetch_postgres().await?;
        assert!(!content.is_empty(), "downloaded content should not be empty");
        Ok(())
    }

    /// Verify that every bundled `PG_Vxx` constant can actually be downloaded
    /// for the compile-time platform.
    ///
    /// Each version is fetched in full and the byte count is printed.  This
    /// test is marked `#[ignore]` because it downloads several hundred MB and
    /// should only be run explicitly:
    ///
    /// ```text
    /// cargo test --features rt_tokio -- --ignored all_versions_downloadable --nocapture
    /// ```
    ///
    /// Maven Central returns a tiny HTML error page with HTTP 200 for missing
    /// artifacts, so a 1 MB minimum is enforced to detect that case.
    ///
    /// **Platform notes:**
    /// - `darwin-arm64v8` (Apple Silicon): binaries exist from PG 14 onward.
    ///   PG 10–13 are excluded on that target via `#[cfg]`.
    /// - All other platforms: all constants are tested.
    #[tokio::test]
    #[ignore]
    async fn all_versions_downloadable() -> Result<()> {
        // PG 10–13 were released before zonky added darwin-arm64v8 support.
        #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
        let versions: &[(&str, PostgresVersion)] = &[
            ("PG_V10", PG_V10),
            ("PG_V11", PG_V11),
            ("PG_V12", PG_V12),
            ("PG_V13", PG_V13),
            ("PG_V14", PG_V14),
            ("PG_V15", PG_V15),
            ("PG_V16", PG_V16),
            ("PG_V17", PG_V17),
            ("PG_V18", PG_V18),
        ];
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        let versions: &[(&str, PostgresVersion)] = &[
            ("PG_V14", PG_V14),
            ("PG_V15", PG_V15),
            ("PG_V16", PG_V16),
            ("PG_V17", PG_V17),
            ("PG_V18", PG_V18),
        ];

        for (name, version) in versions {
            let settings = PgFetchSettings {
                version: *version,
                ..Default::default()
            };
            let bytes = settings.fetch_postgres().await?;
            println!("{name} ({}): {} bytes", version.0, bytes.len());
            assert!(
                bytes.len() > 1_000_000,
                "{name} ({}) returned only {} bytes — likely missing for platform '{}'",
                version.0,
                bytes.len(),
                settings.platform(),
            );
        }
        Ok(())
    }
}
