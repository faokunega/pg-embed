//!
//! Enums
//!

use std::fmt;

use crate::command_executor::ProcessStatus;
use crate::pg_errors::Error;

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

impl fmt::Display for PgAuthMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PgAuthMethod::Plain => write!(f, "password"),
            PgAuthMethod::MD5 => write!(f, "md5"),
            PgAuthMethod::ScramSha256 => write!(f, "scram-sha-256"),
        }
    }
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

impl ProcessStatus<PgServerStatus, Error> for PgProcessType {
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

    fn error_type(&self) -> Error {
        match self {
            PgProcessType::InitDb => Error::PgInitFailure,
            PgProcessType::StartDb => Error::PgStartFailure,
            PgProcessType::StopDb => Error::PgStopFailure,
        }
    }

    fn timeout_error(&self) -> Error {
        Error::PgTimedOutError
    }

    fn wrap_error<E: std::error::Error + Sync + Send + 'static>(
        &self,
        error: E,
        message: Option<String>,
    ) -> Error {
        Error::PgError(error.to_string(), message.unwrap_or_default())
    }
}

impl fmt::Display for PgProcessType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PgProcessType::InitDb => write!(f, "initdb"),
            PgProcessType::StartDb => write!(f, "start"),
            PgProcessType::StopDb => write!(f, "stop"),
        }
    }
}

/// The operation systems enum
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum OperationSystem {
    /// macOS
    Darwin,
    /// Windows
    Windows,
    /// Linux (glibc)
    Linux,
    /// Alpine Linux (musl)
    AlpineLinux,
}

impl fmt::Display for OperationSystem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OperationSystem::Darwin => write!(f, "darwin"),
            OperationSystem::Windows => write!(f, "windows"),
            OperationSystem::Linux => write!(f, "linux"),
            OperationSystem::AlpineLinux => write!(f, "linux"),
        }
    }
}

#[allow(clippy::derivable_impls)]
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
    /// x86_64
    Amd64,
    /// 32-bit x86
    I386,
    /// ARMv6 (32-bit)
    Arm32v6,
    /// ARMv7 (32-bit)
    Arm32v7,
    /// AArch64 / ARMv8 (64-bit)
    Arm64v8,
    /// POWER little-endian 64-bit
    Ppc64le,
}

impl fmt::Display for Architecture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Architecture::Amd64 => write!(f, "amd64"),
            Architecture::I386 => write!(f, "i386"),
            Architecture::Arm32v6 => write!(f, "arm32v6"),
            Architecture::Arm32v7 => write!(f, "arm32v7"),
            Architecture::Arm64v8 => write!(f, "arm64v8"),
            Architecture::Ppc64le => write!(f, "ppc64le"),
        }
    }
}

#[allow(clippy::derivable_impls)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operation_system_display() {
        assert_eq!(OperationSystem::Darwin.to_string(), "darwin");
        assert_eq!(OperationSystem::Windows.to_string(), "windows");
        assert_eq!(OperationSystem::Linux.to_string(), "linux");
        assert_eq!(OperationSystem::AlpineLinux.to_string(), "linux");
    }

    #[test]
    fn test_architecture_display() {
        assert_eq!(Architecture::Amd64.to_string(), "amd64");
        assert_eq!(Architecture::I386.to_string(), "i386");
        assert_eq!(Architecture::Arm32v6.to_string(), "arm32v6");
        assert_eq!(Architecture::Arm32v7.to_string(), "arm32v7");
        assert_eq!(Architecture::Arm64v8.to_string(), "arm64v8");
        assert_eq!(Architecture::Ppc64le.to_string(), "ppc64le");
    }
}
