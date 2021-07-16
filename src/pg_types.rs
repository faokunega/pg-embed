use crate::pg_errors::PgEmbedError;
use std::cell::Cell;
use tokio::process::Command;

pub type PgResult<T> = Result<T, PgEmbedError>;
pub type PgCommand = Box<Cell<Command>>;
pub type PgCommandSync = Box<Cell<std::process::Command>>;