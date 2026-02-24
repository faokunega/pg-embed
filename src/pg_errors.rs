//! Error types and [`Result`] alias for pg-embed.

/// Convenience alias so every fallible function can write `Result<T>` instead
/// of `Result<T, Error>`.
pub type Result<T> = std::result::Result<T, Error>;

/// All errors that pg-embed can produce.
///
/// Every variant maps to a distinct failure mode in the library.  Variants that
/// carry a `String` field contain a human-readable message from the underlying
/// OS or library call that caused the failure; this is always the `.to_string()`
/// of the original error.
#[derive(thiserror::Error, Debug, Clone, PartialEq)]
pub enum Error {
    /// The download URL for PostgreSQL binaries could not be constructed.
    ///
    /// Typically caused by the OS cache directory being unavailable or an
    /// unsupported platform combination.
    #[error("Invalid PostgreSQL binaries download URL.")]
    InvalidPgUrl,

    /// The downloaded file is not a valid PostgreSQL binaries package.
    ///
    /// Raised when the ZIP archive cannot be opened or does not contain the
    /// expected `.xz`-compressed tarball.
    #[error("Invalid PostgreSQL binaries package.")]
    InvalidPgPackage,

    /// A file write operation failed.
    ///
    /// The inner string is the OS error message (e.g. `Permission denied`).
    #[error("Could not write to file: {0}")]
    WriteFileError(String),

    /// A file read or existence-check operation failed.
    ///
    /// The inner string is the OS error message.
    #[error("Could not read file: {0}")]
    ReadFileError(String),

    /// A directory could not be created.
    ///
    /// The inner string is the OS error message.
    #[error("Could not create directory: {0}")]
    DirCreationError(String),

    /// XZ decompression or tar extraction of the PostgreSQL binaries failed.
    #[error("Failed to unpack PostgreSQL binaries.")]
    UnpackFailure,

    /// `pg_ctl start` exited with a non-zero status.
    #[error("PostgreSQL could not be started.")]
    PgStartFailure,

    /// `pg_ctl stop` exited with a non-zero status.
    #[error("PostgreSQL could not be stopped.")]
    PgStopFailure,

    /// `initdb` exited with a non-zero status.
    #[error("PostgreSQL could not be initialized.")]
    PgInitFailure,

    /// Removal of the database directory or password file failed.
    ///
    /// The inner string is the OS error message.
    #[error("Clean up error: {0}")]
    PgCleanUpFailure(String),

    /// Removal of the cached binaries directory failed.
    ///
    /// The inner string is the OS error message.
    #[error("Purging error: {0}")]
    PgPurgeFailure(String),

    /// A buffered I/O read from a process stream failed unexpectedly.
    #[error("Buffer read error.")]
    PgBufferReadError,

    /// A mutex or async lock could not be acquired.
    #[error("Lock error.")]
    PgLockError,

    /// Spawning or waiting on a child process failed.
    #[error("Child process error.")]
    PgProcessError,

    /// A `pg_ctl` or `initdb` call exceeded its configured timeout.
    ///
    /// See [`crate::postgres::PgSettings::timeout`].
    #[error("Operation timed out.")]
    PgTimedOutError,

    /// A `tokio::task::spawn_blocking` join failed.
    ///
    /// The inner string is the [`tokio::task::JoinError`] message.
    #[error("Task join error: {0}")]
    PgTaskJoinError(String),

    /// A generic error wrapper used internally to attach context.
    ///
    /// The first field is the original error message; the second is a
    /// human-readable context string (e.g. `"spawn_blocking join error"`).
    #[error("PgError: {0}, {1}")]
    PgError(String, String),

    /// The HTTP download of the PostgreSQL binaries JAR failed.
    ///
    /// The inner string is the `reqwest` error message.
    #[error("PostgreSQL binaries download failure: {0}")]
    DownloadFailure(String),

    /// Converting the HTTP response body to bytes failed.
    ///
    /// The inner string is the `reqwest` error message.
    #[error("Request response bytes conversion failure: {0}")]
    ConversionFailure(String),

    /// An internal MPSC channel send failed because the receiver was dropped.
    #[error("Channel send error.")]
    SendFailure,

    /// A sqlx query or connection operation failed.
    ///
    /// The inner string is the sqlx error message.
    /// Only produced when the `rt_tokio_migrate` feature is enabled.
    #[error("SQLx query error: {0}")]
    SqlQueryError(String),

    /// Running sqlx migrations failed.
    ///
    /// The inner string is the sqlx migrator error message.
    /// Only produced when the `rt_tokio_migrate` feature is enabled.
    #[error("Migration error: {0}")]
    MigrationError(String),
}
