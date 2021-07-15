//!
//! Fetch postgresql binaries
//!
//! Download and unpack postgresql binaries
//!

use bytes::Bytes;
use futures::TryFutureExt;
use reqwest::Response;

use crate::pg_enums::{Architecture, OperationSystem};
use crate::pg_errors::PgEmbedError;

/// Postgresql version struct (simple version wrapper)
pub struct PostgresVersion(
    pub &'static str,
);

/// Latest postgres version 13
pub const PG_V13: PostgresVersion =
    PostgresVersion("13.2.0");
/// Latest postgres version 12
pub const PG_V12: PostgresVersion =
    PostgresVersion("12.6.0");
/// Latest pstgres version 11
pub const PG_V11: PostgresVersion =
    PostgresVersion("11.11.0");
/// Latest postgres version 10
pub const PG_V10: PostgresVersion =
    PostgresVersion("10.16.0");
/// Latest postgres version 9
pub const PG_V9: PostgresVersion =
    PostgresVersion("9.6.21");

/// Settings that determine the postgres binary to be fetched
pub struct PgFetchSettings {
    /// The repository host
    pub host: String,
    /// The operation system
    pub operating_system:
    OperationSystem,
    /// The cpu architecture
    pub architecture: Architecture,
    /// The postgresql version
    pub version: PostgresVersion,
}

impl Default for PgFetchSettings {
    fn default() -> Self {
        PgFetchSettings {
            host: "https://repo1.maven.org".to_string(),
            operating_system: OperationSystem::default(),
            architecture: Architecture::default(),
            version: PG_V13,
        }
    }
}

impl PgFetchSettings {
    /// The platform string (*needed to determine the download path*)
    pub fn platform(&self) -> String {
        let os = self
            .operating_system
            .to_string();
        let arch =
            if self.operating_system == OperationSystem::AlpineLinux {
                format!("{}-{}", self.architecture.to_string(), "alpine")
            } else { self.architecture.to_string() };
        format!("{}-{}", os, arch)
    }

    ///
    /// Fetch postgres binaries
    ///
    /// Returns the data of the downloaded binary in an `Ok([u8])` on success, otherwise returns an error.
    ///
    pub async fn fetch_postgres(&self) -> Result<Box<Bytes>, PgEmbedError>
    {
        let platform = &self.platform();
        let version = self.version.0;
        let download_url = format!(
            "{}/maven2/io/zonky/test/postgres/embedded-postgres-binaries-{}/{}/embedded-postgres-binaries-{}-{}.jar",
            &self.host,
            &platform,
            version,
            &platform,
            version);
        let response: Response =
            reqwest::get(download_url).map_err(|e|
                { PgEmbedError::DownloadFailure(e) })
                .await?;

        let content: Bytes = response.bytes().map_err(|e| PgEmbedError::ConversionFailure(e))
            .await?;

        Ok(Box::new(content))
    }
}


