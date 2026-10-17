//! The TPM 2.0, the optional further factor.

pub mod device_secret;
pub mod emulator;
#[cfg(test)]
pub mod test_support;
// Same layout as installation/installation.rs.
#[allow(clippy::module_inception)]
pub mod tpm;
