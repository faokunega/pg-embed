use std::cell::Cell;

/// Synchronous pg_ctl command type.
///
/// `Cell` provides interior mutability so the synchronous `pg_ctl stop` command
/// can be configured and spawned inside the `Drop` implementation without requiring
/// a mutable reference to the surrounding struct.
pub type PgCommandSync = Box<Cell<std::process::Command>>;
