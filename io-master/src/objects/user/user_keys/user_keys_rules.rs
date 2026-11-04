//! The rules of the keys of a user: the copy wrapped with the password, the copy wrapped
//! with the recovery code, the code itself, both made anew together.

pub mod password;
pub mod recovery;
pub mod recovery_code;
pub mod renewal;
