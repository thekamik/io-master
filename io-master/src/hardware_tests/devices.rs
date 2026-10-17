//! The scan of the serial ports.

// The file has the name of its folder by choice of the project layout
// (to be renamed in step 40); clippy would prefer another name.
#[allow(clippy::module_inception)]
pub mod devices;
pub mod port_property;
pub mod scan_settings;
