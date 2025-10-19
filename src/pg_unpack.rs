//!
//! Unpack postgresql binaries
//!
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;
use tar::Archive;
use xz2::read::XzDecoder;
use zip::ZipArchive;

use crate::pg_errors::{PgEmbedError, PgEmbedErrorType};
use crate::pg_types::PgResult;

///
/// Unzip the postgresql txz file
///
/// Returns `Ok(PathBuf(txz_file_path))` file path of the txz archive on success, otherwise returns an error.
///
fn unzip_txz(zip_file_path: &PathBuf, cache_dir: &PathBuf) -> Result<PathBuf, PgEmbedError> {
    let zip_file = File::open(zip_file_path).map_err(|e| PgEmbedError {
        error_type: PgEmbedErrorType::ReadFileError,
        source: Some(Box::new(e)),
        message: Some(format!(
            "Could not read zip file {}",
            zip_file_path.display()
        )),
    })?;
    let mut zip_archive = ZipArchive::new(zip_file).map_err(|e| PgEmbedError {
        error_type: PgEmbedErrorType::WriteFileError,
        source: Some(Box::new(e)),
        message: Some(format!(
            "Could not write zip archive {}",
            zip_file_path.display()
        )),
    })?;

    for i in 0..zip_archive.len() {
        let mut file = zip_archive.by_index(i).map_err(|e| PgEmbedError {
            error_type: PgEmbedErrorType::ReadFileError,
            source: Some(Box::new(e)),
            message: Some(format!(
                "Could not read zip file {}",
                zip_file_path.display()
            )),
        })?;
        if file.name().ends_with(".txz") {
            let txz_path = cache_dir.join(file.name());
            let txz_file = File::create(&txz_path).map_err(|e| PgEmbedError {
                error_type: PgEmbedErrorType::WriteFileError,
                source: Some(Box::new(e)),
                message: Some(format!(
                    "Could not create txz in cache {}",
                    zip_file_path.display()
                )),
            })?;
            std::io::copy(&mut file, &mut BufWriter::new(&txz_file)).map_err(|e| PgEmbedError {
                error_type: PgEmbedErrorType::ReadFileError,
                source: Some(Box::new(e)),
                message: Some(format!(
                    "Could not write to txz file in cache {}",
                    zip_file_path.display()
                )),
            })?;
            return Ok(txz_path);
        }
    }

    Err(PgEmbedError {
        error_type: PgEmbedErrorType::InvalidPgPackage,
        source: None,
        message: Some(String::from("No PostgreSQL txz found in zip")),
    })
}

///
/// Decompress the postgresql txz file
///
/// Returns `Ok(PathBuf(tar_file_path))` (*the file path to the postgresql tar file*) on success, otherwise returns an error.
///
fn decompress_xz(zip_file_path: &PathBuf) -> Result<PathBuf, PgEmbedError> {
    let xz_file = File::open(zip_file_path).map_err(|e| PgEmbedError {
        error_type: PgEmbedErrorType::ReadFileError,
        source: Some(Box::new(e)),
        message: Some(format!(
            "Could not read zip file {}",
            zip_file_path.display()
        )),
    })?;
    let xz_decoder = XzDecoder::new(xz_file);
    let target_path = zip_file_path.with_extension("tar");
    let tar_file = File::create(&target_path).map_err(|e| PgEmbedError {
        error_type: PgEmbedErrorType::WriteFileError,
        source: Some(Box::new(e)),
        message: Some(format!(
            "Could not write tar file to {}",
            target_path.display()
        )),
    })?;
    std::io::copy(
        &mut BufReader::new(xz_decoder),
        &mut BufWriter::new(&tar_file),
    )
    .map_err(|e| PgEmbedError {
        error_type: PgEmbedErrorType::WriteFileError,
        source: Some(Box::new(e)),
        message: Some(format!(
            "Could not write tar file to {}",
            target_path.display()
        )),
    })?;
    Ok(target_path)
}

///
/// Unpack the postgresql tar file
///
/// Returns `Ok(())` on success, otherwise returns an error.
///
fn decompress_tar(file_path: &PathBuf, cache_dir: &PathBuf) -> Result<(), PgEmbedError> {
    let tar_file = File::open(file_path).map_err(|e| PgEmbedError {
        error_type: PgEmbedErrorType::ReadFileError,
        source: Some(Box::new(e)),
        message: None,
    })?;
    let mut archive = Archive::new(tar_file);
    archive.unpack(cache_dir).map_err(|e| PgEmbedError {
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
    let txz_file_path = unzip_txz(zip_file_path, cache_dir)?;
    let tar_file_path = decompress_xz(&txz_file_path)?;
    tokio::fs::remove_file(txz_file_path)
        .await
        .map_err(|e| PgEmbedError {
            error_type: PgEmbedErrorType::PgCleanUpFailure,
            source: Some(Box::new(e)),
            message: None,
        })?;
    decompress_tar(&tar_file_path, cache_dir)?;
    tokio::fs::remove_file(tar_file_path)
        .await
        .map_err(|e| PgEmbedError {
            error_type: PgEmbedErrorType::PgCleanUpFailure,
            source: Some(Box::new(e)),
            message: None,
        })?;
    Ok(())
}
