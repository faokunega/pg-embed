//!
//! Enums
//!

use std::error::Error;

use crate::command_executor::ProcessStatus;
use crate::pg_errors::{PgEmbedError, PgEmbedErrorType};

///
/// Postgresql authentication method
///
/// Choose between plain password, md5 or scram_sha_256 authentication.
/// Scram_sha_256 authentication is only available on postgresql versions >= 11
///
pub enum PgAuthMethod {
    /// plain-text
    Plain,
    /// md5
    MD5,
    /// scram_sha_256
    ScramSha256,
}

///
/// Postgresql server status
///
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PgServerStatus {
    /// Postgres uninitialized
    Uninitialized,
    /// Initialization process running
    Initializing,
    /// Initialization process finished
    Initialized,
    /// Postgres server process starting
    Starting,
    /// Postgres server process started
    Started,
    /// Postgres server process stopping
    Stopping,
    /// Postgres server process stopped
    Stopped,
    /// Postgres failure
    Failure,
}

///
/// Postgesql process type
///
/// Used internally for distinguishing processes being executed
///
pub enum PgProcessType {
    /// initdb process
    InitDb,
    /// pg_ctl start process
    StartDb,
    /// pg_ctl stop process
    StopDb,
}

impl ProcessStatus<PgServerStatus, PgEmbedError> for PgProcessType {
    fn status_entry(&self) -> PgServerStatus {
        match self {
            PgProcessType::InitDb => PgServerStatus::Initializing,
            PgProcessType::StartDb => PgServerStatus::Starting,
            PgProcessType::StopDb => PgServerStatus::Stopping,
        }
    }

    fn status_exit(&self) -> PgServerStatus {
        match self {
            PgProcessType::InitDb => PgServerStatus::Initialized,
            PgProcessType::StartDb => PgServerStatus::Started,
            PgProcessType::StopDb => PgServerStatus::Stopped,
        }
    }

    fn error_type(&self) -> PgEmbedError {
        match self {
            PgProcessType::InitDb => PgEmbedError {
                error_type: PgEmbedErrorType::PgInitFailure,
                source: None,
                message: None,
            },
            PgProcessType::StartDb => PgEmbedError {
                error_type: PgEmbedErrorType::PgStartFailure,
                source: None,
                message: None,
            },
            PgProcessType::StopDb => PgEmbedError {
                error_type: PgEmbedErrorType::PgStopFailure,
                source: None,
                message: None,
            },
        }
    }

    fn wrap_error<E: Error + Sync + Send + 'static>(
        &self,
        error: E,
        message: Option<String>,
    ) -> PgEmbedError {
        PgEmbedError {
            error_type: PgEmbedErrorType::PgError,
            source: Some(Box::new(error)),
            message,
        }
    }
}

impl ToString for PgProcessType {
    fn to_string(&self) -> String {
        match self {
            PgProcessType::InitDb => "initdb".to_string(),
            PgProcessType::StartDb => "start".to_string(),
            PgProcessType::StopDb => "stop".to_string(),
        }
    }
}

/// The operation systems enum
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum OperationSystem {
    Darwin,
    Windows,
    Linux,
    AlpineLinux,
}

impl ToString for OperationSystem {
    fn to_string(&self) -> String {
        match &self {
            OperationSystem::Darwin => "darwin".to_string(),
            OperationSystem::Windows => "windows".to_string(),
            OperationSystem::Linux => "linux".to_string(),
            OperationSystem::AlpineLinux => "linux".to_string(),
        }
    }
}

impl Default for OperationSystem {
    fn default() -> Self {
        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        {
            OperationSystem::Darwin
        }

        #[cfg(target_os = "linux")]
        {
            OperationSystem::Linux
        }

        #[cfg(target_os = "windows")]
        {
            OperationSystem::Windows
        }
    }
}

/// The cpu architectures enum
#[derive(Debug, PartialEq, Copy, Clone)]
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
            Architecture::Amd64 => "amd64".to_string(),
            Architecture::I386 => "i386".to_string(),
            Architecture::Arm32v6 => "arm32v6".to_string(),
            Architecture::Arm32v7 => "arm32v7".to_string(),
            Architecture::Arm64v8 => "arm64v8".to_string(),
            Architecture::Ppc64le => "ppc64le".to_string(),
        }
    }
}

impl Default for Architecture {
    fn default() -> Self {
        #[cfg(not(any(
            target_arch = "x86",
            target_arch = "arm",
            target_arch = "aarch64",
            target_arch = "powerpc64"
        )))]
        {
            Architecture::Amd64
        }

        #[cfg(target_arch = "x86")]
        {
            Architecture::I386
        }

        #[cfg(target_arch = "arm")]
        {
            Architecture::Arm32v7
        }

        #[cfg(target_arch = "aarch64")]
        {
            Architecture::Arm64v8
        }

        #[cfg(target_arch = "powerpc64")]
        {
            Architecture::Ppc64le
        }
    }
}

/// The postgresql binaries acquisition status
#[derive(Copy, Clone, PartialEq)]
pub enum PgAcquisitionStatus {
    /// Acquiring postgresql binaries
    InProgress,
    /// Finished acquiring postgresql binaries
    Finished,
    /// No acquisition
    Undefined,
}
