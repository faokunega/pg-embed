use crate::pg_errors::PgEmbedError;
use std::cell::Cell;

pub type PgResult<T> = Result<T, PgEmbedError>;
pub type PgCommandSync = Box<Cell<std::process::Command>>;
