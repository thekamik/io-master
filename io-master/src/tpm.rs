//! The TPM 2.0, the optional further factor.

pub mod device_secret;
pub mod emulator;
#[cfg(test)]
pub mod test_support;
// The file has the name of its folder by choice of the project layout
// (to be renamed in step 40); clippy would prefer another name.
#[allow(clippy::module_inception)]
pub mod tpm;
