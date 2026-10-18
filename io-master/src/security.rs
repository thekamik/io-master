//! The building blocks of protection: random numbers from the system,
//! and the TPM 2.0, the optional further factor (step 40 of
//! docs/todo.md). Later the keyring of the Linux account (step 13).
//! The encryption of the databases is in `db/encryption/`.

pub mod random;
pub mod tpm;
