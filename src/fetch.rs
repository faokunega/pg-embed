use archiver_rs::{
    Archive, Compressed,
};
use futures::future::BoxFuture;
use futures::{TryFutureExt};
use std::borrow::Borrow;
use std::path::{PathBuf, Path};
use crate::errors::PgEmbedError;
use reqwest::Response;
use tokio::io::AsyncWriteExt;

/// The operation systems enum
#[derive(Debug, PartialEq)]
pub enum OperationSystem {
    Darwin,
    Windows,
    Linux,
    AlpineLinux,
}

impl ToString for OperationSystem {
    fn to_string(&self) -> String {
        match &self {
            OperationSystem::Darwin => { "darwin".to_string() }
            OperationSystem::Windows => { "windows".to_string() }
            OperationSystem::Linux => { "linux".to_string() }
            OperationSystem::AlpineLinux => { "linux".to_string() }
        }
    }
}

/// The cpu architectures enum
#[derive(Debug, PartialEq)]
pub enum Architecture {
    Amd64,
    I386,
    Arm32v6,
    Arm32v7,
    Arm64v8,
    Ppc64le,
}

impl ToString for Architecture {
    fn to_string(&self) -> String {
        match &self {
            Architecture::Amd64 => {
                "amd64".to_string()
            }
            Architecture::I386 => {
                "i386".to_string()
            }
            Architecture::Arm32v6 => {
                "arm32v6".to_string()
            }
            Architecture::Arm32v7 => {
                "arm32v7".to_string()
            }
            Architecture::Arm64v8 => {
                "arm64v8".to_string()
            }
            Architecture::Ppc64le => {
                "ppc64le".to_string()
            }
        }
    }
}

/// Postgresql version struct (simple version wrapper)
pub struct PostgresVersion(
    &'static str,
);

/// Postgres version 13
pub const PG_V13: PostgresVersion =
    PostgresVersion("13.1.0-1");
/// Postgres version 12
pub const PG_V12: PostgresVersion =
    PostgresVersion("12.1.0-1");
/// Postgres version 11
pub const PG_V11: PostgresVersion =
    PostgresVersion("11.6.0-1");
/// Postgres version 10
pub const PG_V10: PostgresVersion =
    PostgresVersion("10.11.0-1");
/// Postgres version 9
pub const PG_V9: PostgresVersion =
    PostgresVersion("9.6.16-1");

/// Settings that determine the postgres binary to be fetched
pub struct FetchSettings {
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

impl Default for FetchSettings {
    fn default() -> Self {
        FetchSettings {
            host: "https://repo1.maven.org".to_string(),
            operating_system:
            OperationSystem::Darwin,
            architecture:
            Architecture::Amd64,
            version: PG_V13,
        }
    }
}

impl FetchSettings {
    /// The platform string (*needed to determine the download path*)
    fn platform(&self) -> String {
        let os = self
            .operating_system
            .to_string();
        let arch =
            if self.operating_system == OperationSystem::AlpineLinux {
                format!("{}-{}", self.architecture.to_string(), "alpine")
            } else { self.architecture.to_string() };
        format!("{}-{}", os, arch)
    }
}

///
/// Fetch a postgres binary
///
/// The [settings](FetchSettings) parameter determines which binary to load.
/// Returns the file name of the downloaded binary in an `Ok(String)` on success, otherwise returns an error.
///
pub async fn fetch_postgres(
    settings: &FetchSettings, executable_path: &str,
) -> Result<String, PgEmbedError>
{
    // download binary
    let platform =
        settings.platform();
    let file_name = format!(
        "{}-{}.zip",
        platform,
        &settings.version.0
    );
    let file_path = format!(
        "{}/{}",
        &executable_path,
        &file_name,
    );
    let path =
        Path::new(&file_path);
    if !path.exists() {
        let download_url = format!(
            "{}/maven2/io/zonky/test/postgres/embedded-postgres-binaries-{}/{}/embedded-postgres-binaries-{}-{}.jar",
            &settings.host,
            &platform,
            &settings.version.0,
            &platform,
            &settings.version.0);
        let mut response: Response =
            reqwest::get(download_url).map_err(|e|
                { PgEmbedError::DownloadFailure(e) })
                .await?;

        // write binary to file
        tokio::fs::create_dir_all(
            executable_path,
        ).map_err(|e| PgEmbedError::DirCreationError(e))
            .await?;
        let mut file: tokio::fs::File =
            tokio::fs::File::create(&path).map_err(|e| PgEmbedError::WriteFileError(e)).await?;
        let content = response.bytes().map_err(|e| PgEmbedError::ConversionFailure(e))
            .await?;
        file.write_all(&content).map_err(|e| PgEmbedError::WriteFileError(e))
            .await?;
    }

    Ok(file_name)
}

///
/// Unzip the postgresql txz file
///
/// Returns `Ok(String)` file name of the txz archive on success, otherwise returns an error.
///
fn unzip_txz(file_path: &str, executables_path: &str) -> Result<String, PgEmbedError> {
    let path = std::path::Path::new(
        &file_path,
    );
    let mut zip =
        archiver_rs::Zip::open(&path).map_err(|e| PgEmbedError::ReadFileError(e))?;
    let file_name = zip.files().map_err(|e| PgEmbedError::UnpackFailure(e))?
        .into_iter()
        .find(|name| {
            name.ends_with(".txz")
        });
    match file_name {
        Some(file_name) => {
            // decompress zip
            let target_name = format!("{}/{}", &executables_path, &file_name);
            let target_path =
                std::path::Path::new(
                    &target_name,
                );
            zip.extract_single(
                &target_path,
                file_name.clone(),
            ).map_err(|e| PgEmbedError::UnpackFailure(e))?;
            Ok(target_name)
        }
        None => { Err(PgEmbedError::InvalidPgPackage("not postgresql txz in zip".to_string())) }
    }
}

///
/// Decompress the postgresql txz file
///
/// Returns `Ok(String)` (*the relative path to the postgresql tar file*) on success, otherwise returns an error.
///
fn decompress_xz(file_path: &str) -> Result<String, PgEmbedError> {
    let target_path =
        std::path::Path::new(
            &file_path,
        );
    let mut xz =
        archiver_rs::Xz::open(
            &target_path,
        ).map_err(|e| PgEmbedError::ReadFileError(e))?;
    let package_path = file_path.strip_suffix(".txz").unwrap();
    let target_name = format!("{}.tar", package_path);
    let target_path =
        std::path::Path::new(
            &target_name,
        );
    xz.decompress(target_path).map_err(|e| PgEmbedError::UnpackFailure(e))?;
    Ok(target_name)
}

///
/// Unpack the postgresql tar file
///
/// Returns `Ok(())` on success, otherwise returns an error.
///
fn decompress_tar(file_path: &str, executables_path: &str) -> Result<(), PgEmbedError> {
    let target = std::path::Path::new(&file_path);
    let mut tar =
        archiver_rs::Tar::open(
            &target,
        ).map_err(|e| PgEmbedError::ReadFileError(e))?;

    let target_path =
        std::path::Path::new(
            &executables_path,
        );

    tar.extract(target_path).map_err(|e| PgEmbedError::UnpackFailure(e))?;

    Ok(())
}

///
/// Unpack the postgresql executables
///
/// Returns `Ok(())` on success, otherwise returns an error.
///
pub async fn unpack_postgres(
    file_name: &str, executables_path: &str,
) -> Result<(), PgEmbedError> {
    let file_path = format!("{}/{}", executables_path, file_name);
    let txz_file_path = unzip_txz(&file_path, &executables_path)?;
    let tar_file_path = decompress_xz(&txz_file_path)?;
    tokio::fs::remove_file(txz_file_path).map_err(|e| PgEmbedError::PgCleanUpFailure(e)).await?;
    decompress_tar(&tar_file_path, &executables_path);
    tokio::fs::remove_file(tar_file_path).map_err(|e| PgEmbedError::PgCleanUpFailure(e)).await?;
    Ok(())
}
