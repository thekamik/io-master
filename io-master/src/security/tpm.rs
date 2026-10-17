//! The TPM 2.0, the optional further factor (`docs/architecture.md`,
//! section 5.1).
//!
//! ```text
//! chip.rs            talking to the TPM: sealing, the marker of the installation
//! device_secret.rs   the secret of the device, sealed by the TPM
//! emulator.rs        the emulator swtpm, for development and tests
//! ```

pub mod chip;
pub mod device_secret;
pub mod emulator;
#[cfg(test)]
pub mod test_support;
