//!
//! Unpack postgresql binaries
//!
use std::path::PathBuf;

use archiver_rs::{Archive, Compressed};
use futures::TryFutureExt;

use crate::pg_errors::{PgEmbedError, PgEmbedErrorType};
use crate::pg_types::PgResult;

///
/// Unzip the postgresql txz file
///
/// Returns `Ok(PathBuf(txz_file_path))` file path of the txz archive on success, otherwise returns an error.
///
fn unzip_txz(zip_file_path: &PathBuf, cache_dir: &PathBuf) -> Result<PathBuf, PgEmbedError> {
    let mut zip = archiver_rs::Zip::open(zip_file_path.as_path()).map_err(|e| PgEmbedError {
        error_type: PgEmbedErrorType::ReadFileError,
        source: Some(Box::new(e)),
        message: Some(format!(
            "Could not read zip file {}",
            zip_file_path.display()
        )),
    })?;
    let file_name = zip
        .files()
        .map_err(|e| PgEmbedError {
            error_type: PgEmbedErrorType::UnpackFailure,
            source: Some(Box::new(e)),
            message: None,
        })?
        .into_iter()
        .find(|name| name.ends_with(".txz"));
    match file_name {
        Some(file_name) => {
            // decompress zip
            let mut target_path = cache_dir.clone();
            target_path.push(&file_name);
            zip.extract_single(&target_path.as_path(), file_name.clone())
                .map_err(|e| PgEmbedError {
                    error_type: PgEmbedErrorType::UnpackFailure,
                    source: Some(Box::new(e)),
                    message: None,
                })?;
            Ok(target_path)
        }
        None => Err(PgEmbedError {
            error_type: PgEmbedErrorType::InvalidPgPackage,
            source: None,
            message: Some(String::from("no postgresql txz in zip")),
        }),
    }
}

///
/// Decompress the postgresql txz file
///
/// Returns `Ok(PathBuf(tar_file_path))` (*the file path to the postgresql tar file*) on success, otherwise returns an error.
///
fn decompress_xz(file_path: &PathBuf) -> Result<PathBuf, PgEmbedError> {
    let mut xz = archiver_rs::Xz::open(file_path.as_path()).map_err(|e| PgEmbedError {
        error_type: PgEmbedErrorType::ReadFileError,
        source: Some(Box::new(e)),
        message: None,
    })?;
    // rename file path suffix from .txz to .tar
    let target_path = file_path.with_extension(".tar");
    xz.decompress(&target_path.as_path())
        .map_err(|e| PgEmbedError {
            error_type: PgEmbedErrorType::UnpackFailure,
            source: Some(Box::new(e)),
            message: None,
        })?;
    Ok(target_path)
}

///
/// Unpack the postgresql tar file
///
/// Returns `Ok(())` on success, otherwise returns an error.
///
fn decompress_tar(file_path: &PathBuf, cache_dir: &PathBuf) -> Result<(), PgEmbedError> {
    let mut tar = archiver_rs::Tar::open(&file_path.as_path()).map_err(|e| PgEmbedError {
        error_type: PgEmbedErrorType::ReadFileError,
        source: Some(Box::new(e)),
        message: None,
    })?;

    tar.extract(cache_dir.as_path()).map_err(|e| PgEmbedError {
        error_type: PgEmbedErrorType::UnpackFailure,
        source: Some(Box::new(e)),
        message: None,
    })?;

    Ok(())
}

///
/// Unpack the postgresql executables
///
/// Returns `Ok(())` on success, otherwise returns an error.
///
pub async fn unpack_postgres(zip_file_path: &PathBuf, cache_dir: &PathBuf) -> PgResult<()> {
    let txz_file_path = unzip_txz(&zip_file_path, &cache_dir)?;
    let tar_file_path = decompress_xz(&txz_file_path)?;
    tokio::fs::remove_file(txz_file_path)
        .map_err(|e| PgEmbedError {
            error_type: PgEmbedErrorType::PgCleanUpFailure,
            source: Some(Box::new(e)),
            message: None,
        })
        .await?;
    let _ = decompress_tar(&tar_file_path, &cache_dir);
    tokio::fs::remove_file(tar_file_path)
        .map_err(|e| PgEmbedError {
            error_type: PgEmbedErrorType::PgCleanUpFailure,
            source: Some(Box::new(e)),
            message: None,
        })
        .await?;
    Ok(())
}
