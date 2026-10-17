//! The installation: its record, its life (first start, removal), its check.

// The file is installation/installation.rs by choice of the
// project layout; clippy would prefer a different name for it.
#[allow(clippy::module_inception)]
pub mod installation;
pub mod lifecycle;
pub mod verify;
