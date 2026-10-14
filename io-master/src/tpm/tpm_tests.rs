//! Unit tests of `tpm.rs`, on the TPM emulator.

use super::*;

use crate::tpm::emulator::Emulator;
use crate::tpm::test_support::TestTpm;

#[test]
fn a_sealed_secret_opens_again() {
    let test = TestTpm::new();
    let mut tpm = test.connect();

    let envelope = tpm.seal(b"thirty-two bytes of a secret....").unwrap();

    // The envelope does not contain the secret in plain form.
    let bytes = envelope.to_bytes();
    assert!(!bytes.windows(10).any(|window| window == b"thirty-two"));

    // A new connection, as after a restart of the program.
    let mut tpm = test.connect();
    let opened = tpm.unseal(&SealedEnvelope::from_bytes(&bytes).unwrap()).unwrap();

    assert_eq!(opened.as_slice(), b"thirty-two bytes of a secret....");
}

#[test]
fn another_tpm_cannot_open_the_envelope() {
    let first = TestTpm::new();
    let second = TestTpm::new();

    let envelope = first.connect().seal(b"secret").unwrap();

    assert!(second.connect().unseal(&envelope).is_err());
}

#[test]
fn a_damaged_envelope_is_refused() {
    for bytes in [&[][..], &[0, 5, 1, 2][..], &[0, 0, 1, 2, 3][..]] {
        assert!(matches!(SealedEnvelope::from_bytes(bytes), Err(TpmError::DamagedEnvelope)));
    }

    let envelope = SealedEnvelope {
        public: vec![1, 2, 3],
        private: vec![4, 5],
    };

    assert_eq!(SealedEnvelope::from_bytes(&envelope.to_bytes()).unwrap(), envelope);
}

#[test]
fn the_marker_is_written_once_and_removed_without_a_trace() {
    let test = TestTpm::new();
    let mut tpm = test.connect();
    let id = [7u8; MARKER_LENGTH];

    assert_eq!(tpm.read_marker().unwrap(), None);

    tpm.write_marker(&id).unwrap();
    assert_eq!(tpm.read_marker().unwrap(), Some(id));

    // A second installation is refused.
    assert!(matches!(tpm.write_marker(&[8u8; MARKER_LENGTH]), Err(TpmError::MarkerExists)));
    assert_eq!(tpm.read_marker().unwrap(), Some(id));

    tpm.remove_marker().unwrap();
    assert_eq!(tpm.read_marker().unwrap(), None);

    // Removing again is not an error.
    tpm.remove_marker().unwrap();
}

#[test]
fn the_marker_and_the_secret_survive_a_restart_of_the_emulator() {
    let test = TestTpm::new();
    let id = [3u8; MARKER_LENGTH];

    test.connect().write_marker(&id).unwrap();
    let envelope = test.connect().seal(b"kept").unwrap();

    // A new emulator on the same state, as at the next start of
    // the program. Starting it stops the one left running.
    let restarted = Emulator::start(test.dir()).unwrap();
    let mut tpm = TpmAccess { port: restarted.port() }.connect().unwrap();

    assert_eq!(tpm.read_marker().unwrap(), Some(id));
    assert_eq!(tpm.unseal(&envelope).unwrap().as_slice(), b"kept");
}
