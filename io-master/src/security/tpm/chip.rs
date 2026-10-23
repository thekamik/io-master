//! Talking to the TPM 2.0.
//!
//! Two things are kept with the help of the TPM
//! (`docs/architecture.md`, sections 3.2 and 5):
//!
//! - **Sealed secrets.** [`Tpm::seal`] gives a secret to the TPM,
//!   which returns it encrypted as a [`SealedEnvelope`]. The
//!   envelope can be stored anywhere: only this TPM can open it
//!   again ([`Tpm::unseal`]). Nothing stays in the TPM.
//! - **The marker of the installation.** A small entry in the
//!   non-volatile memory of the TPM, written once at the first
//!   start ([`Tpm::write_marker`]) and removed when the program is
//!   uninstalled ([`Tpm::remove_marker`]). It holds the identifier
//!   of the installation and tells whether this computer has one.
//!
//! How sealing works
//! -----------------
//! Every TPM has a secret seed that never leaves it. From that
//! seed the TPM derives its *primary key* the same way every time
//! it is asked, so the key is never stored: it is created when
//! needed and removed from the memory of the TPM right after use.
//! A sealed envelope is encrypted under that primary key, which is
//! why only the same TPM can open it.
//!
//! Commands go through an encrypted session, so secrets do not
//! travel as plain text between the program and the TPM.
//!
//! What the program never does: clear the TPM, change its
//! passwords, or bind secrets to the state of the machine (PCR).

// TODO (step 8 of docs/todo.md): the TPM is an optional factor that
//       is not switched on yet; until then only the tests use this
//       file.
#![allow(dead_code)]

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use tss_esapi::attributes::{NvIndexAttributesBuilder, ObjectAttributesBuilder, SessionAttributesBuilder};
use tss_esapi::constants::{CapabilityType, SessionType};
use tss_esapi::handles::{KeyHandle, NvIndexHandle, NvIndexTpmHandle, TpmHandle};
use tss_esapi::interface_types::algorithm::{HashingAlgorithm, PublicAlgorithm};
use tss_esapi::interface_types::key_bits::RsaKeyBits;
use tss_esapi::interface_types::resource_handles::{Hierarchy, NvAuth, Provision};
use tss_esapi::structures::{
    CapabilityData, KeyedHashScheme, MaxNvBuffer, NvPublicBuilder, Private, Public, PublicBuilder,
    PublicKeyedHashParameters, RsaExponent, SensitiveData, SymmetricDefinition, SymmetricDefinitionObject,
};
use tss_esapi::tcti_ldr::{NetworkTPMConfig, TctiNameConf};
use tss_esapi::traits::{Marshall, UnMarshall};
use tss_esapi::{Context, utils};
use zeroize::Zeroizing;

/// The place of the marker in the non-volatile memory of the TPM.
///
/// 0x01000000 to 0x013FFFFF is the range for indexes defined by the
/// owner of the TPM, that is by programs. The last bytes spell
/// "IM", for IO Master.
const MARKER_INDEX: u32 = 0x0100_494D;

/// Size of the marker: the identifier of the installation.
pub const MARKER_LENGTH: usize = 16;

/// Why an operation with the TPM failed.
#[derive(Debug)]
pub enum TpmError {
    /// The emulator `swtpm` is not installed.
    EmulatorMissing,
    /// The emulator could not be started; holds the reason.
    Emulator(String),
    /// The TPM refused or failed a command; holds its message.
    Tpm(String),
    /// The marker is already written: this computer has an
    /// installation.
    MarkerExists,
    /// A sealed envelope does not have the expected form.
    DamagedEnvelope,
    /// A file could not be read or written.
    Io { path: PathBuf, source: io::Error },
}

impl TpmError {
    pub fn io(path: &Path, source: io::Error) -> TpmError {
        TpmError::Io { path: path.to_owned(), source }
    }
}

impl fmt::Display for TpmError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TpmError::EmulatorMissing => formatter.write_str("the TPM emulator swtpm is not installed (package swtpm)"),
            TpmError::Emulator(reason) => write!(formatter, "TPM emulator: {reason}"),
            TpmError::Tpm(message) => write!(formatter, "TPM: {message}"),
            TpmError::MarkerExists => formatter.write_str("this computer already has an installation"),
            TpmError::DamagedEnvelope => formatter.write_str("a sealed secret is damaged"),
            TpmError::Io { path, source } => write!(formatter, "{}: {}", path.display(), source),
        }
    }
}

impl std::error::Error for TpmError {}

impl From<tss_esapi::Error> for TpmError {
    fn from(error: tss_esapi::Error) -> TpmError {
        TpmError::Tpm(error.to_string())
    }
}

/// Where the TPM is reached: today always the emulator on a port
/// of 127.0.0.1 (see `emulator.rs`).
///
/// Cheap to copy. A [`Tpm`] connection cannot move between
/// threads, so every piece of work opens its own with
/// [`TpmAccess::connect`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TpmAccess {
    pub port: u16,
}

impl TpmAccess {
    /// Opens a connection with an encrypted session.
    pub fn connect(self) -> Result<Tpm, TpmError> {
        let address = NetworkTPMConfig::from_str(&format!("host=127.0.0.1,port={}", self.port))?;
        let mut context = Context::new(TctiNameConf::Swtpm(address))?;

        // Parameters of the commands are encrypted (AES-128) in
        // both directions, and every command is authenticated.
        let session = context.start_auth_session(
            None,
            None,
            None,
            SessionType::Hmac,
            SymmetricDefinition::AES_128_CFB,
            HashingAlgorithm::Sha256,
        )?;

        let (attributes, mask) = SessionAttributesBuilder::new().with_decrypt(true).with_encrypt(true).build();

        let session = session.ok_or_else(|| TpmError::Tpm("no session".to_owned()))?;

        context.tr_sess_set_attributes(session, attributes, mask)?;
        context.set_sessions((Some(session), None, None));

        Ok(Tpm { context })
    }
}

/// A secret sealed by the TPM: the two parts the TPM returns. Not
/// secret itself; only the same TPM can open it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SealedEnvelope {
    public: Vec<u8>,
    private: Vec<u8>,
}

impl SealedEnvelope {
    /// The envelope as one block of bytes: the length of the public
    /// part (2 bytes), the public part, the private part.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(2 + self.public.len() + self.private.len());

        bytes.extend_from_slice(&(self.public.len() as u16).to_be_bytes());
        bytes.extend_from_slice(&self.public);
        bytes.extend_from_slice(&self.private);

        bytes
    }

    /// The envelope from [`SealedEnvelope::to_bytes`].
    pub fn from_bytes(bytes: &[u8]) -> Result<SealedEnvelope, TpmError> {
        let (length, rest) = bytes.split_at_checked(2).ok_or(TpmError::DamagedEnvelope)?;
        let length = usize::from(u16::from_be_bytes([length[0], length[1]]));
        let (public, private) = rest.split_at_checked(length).ok_or(TpmError::DamagedEnvelope)?;

        if public.is_empty() || private.is_empty() {
            return Err(TpmError::DamagedEnvelope);
        }

        Ok(SealedEnvelope { public: public.to_vec(), private: private.to_vec() })
    }
}

/// A connection to the TPM.
pub struct Tpm {
    context: Context,
}

impl Tpm {
    /// Creates the primary key: the same key every time, derived
    /// from the seed of the TPM. The caller removes it from the TPM
    /// with `flush_context` when done.
    fn primary_key(&mut self) -> Result<KeyHandle, TpmError> {
        let template = utils::create_restricted_decryption_rsa_public(
            SymmetricDefinitionObject::AES_128_CFB,
            RsaKeyBits::Rsa2048,
            RsaExponent::default(),
        )?;

        Ok(self.context.create_primary(Hierarchy::Owner, template, None, None, None, None)?.key_handle)
    }

    /// Runs `work` with the primary key and removes the key from
    /// the TPM afterwards, also when `work` fails.
    fn with_primary_key<T>(
        &mut self,
        work: impl FnOnce(&mut Context, KeyHandle) -> Result<T, TpmError>,
    ) -> Result<T, TpmError> {
        let key = self.primary_key()?;
        let result = work(&mut self.context, key);

        let _ = self.context.flush_context(key.into());

        result
    }

    /// Seals a secret (at most 128 bytes): only this TPM can open
    /// the envelope again.
    pub fn seal(&mut self, secret: &[u8]) -> Result<SealedEnvelope, TpmError> {
        // A "sealed data object": a keyed-hash object without
        // a scheme holds data instead of a key. It cannot move to
        // another TPM (fixed_tpm) or parent (fixed_parent), and it
        // is not counted against dictionary attacks (no_da), since
        // it has no password.
        let attributes = ObjectAttributesBuilder::new()
            .with_fixed_tpm(true)
            .with_fixed_parent(true)
            .with_no_da(true)
            .with_user_with_auth(true)
            .build()?;

        let template = PublicBuilder::new()
            .with_public_algorithm(PublicAlgorithm::KeyedHash)
            .with_name_hashing_algorithm(HashingAlgorithm::Sha256)
            .with_object_attributes(attributes)
            .with_keyed_hash_parameters(PublicKeyedHashParameters::new(KeyedHashScheme::Null))
            .with_keyed_hash_unique_identifier(Default::default())
            .build()?;

        let data = SensitiveData::try_from(secret.to_vec())?;

        self.with_primary_key(|context, key| {
            let created = context.create(key, template, None, Some(data), None, None)?;

            Ok(SealedEnvelope { public: created.out_public.marshall()?, private: created.out_private.value().to_vec() })
        })
    }

    /// Opens a sealed envelope. Fails when it was sealed by another
    /// TPM or is damaged.
    pub fn unseal(&mut self, envelope: &SealedEnvelope) -> Result<Zeroizing<Vec<u8>>, TpmError> {
        let public = Public::unmarshall(&envelope.public).map_err(|_| TpmError::DamagedEnvelope)?;
        let private = Private::try_from(envelope.private.clone()).map_err(|_| TpmError::DamagedEnvelope)?;

        self.with_primary_key(|context, key| {
            let object = context.load(key, private, public)?;
            let result = context.unseal(object.into());

            let _ = context.flush_context(object.into());

            Ok(Zeroizing::new(result?.value().to_vec()))
        })
    }

    /// The handle of the marker index.
    fn marker_index() -> Result<NvIndexTpmHandle, TpmError> {
        Ok(NvIndexTpmHandle::new(MARKER_INDEX)?)
    }

    /// Tells whether the marker is written, without the error the
    /// TPM library logs when an index that does not exist is
    /// opened: the list of defined indexes is asked instead.
    fn marker_defined(&mut self) -> Result<bool, TpmError> {
        // Without the encrypted session: this command has no
        // parameter that could be encrypted, and the TPM refuses
        // a session that asks for it. Nothing secret is sent.
        let (data, _) = self
            .context
            .execute_without_session(|context| context.get_capability(CapabilityType::Handles, MARKER_INDEX, 1))?;

        let CapabilityData::Handles(handles) = data else {
            return Ok(false);
        };

        let marker = TpmHandle::NvIndex(Self::marker_index()?);

        Ok(handles.into_inner().contains(&marker))
    }

    /// Opens the marker index, which must exist.
    fn open_marker(&mut self) -> Result<NvIndexHandle, TpmError> {
        let handle = self.context.tr_from_tpm_public(TpmHandle::NvIndex(Self::marker_index()?))?;

        Ok(handle.into())
    }

    /// Reads the marker: the identifier of the installation, or
    /// `None` when there is none.
    pub fn read_marker(&mut self) -> Result<Option<[u8; MARKER_LENGTH]>, TpmError> {
        if !self.marker_defined()? {
            return Ok(None);
        }

        let index = self.open_marker()?;
        let data = self.context.nv_read(NvAuth::Owner, index, MARKER_LENGTH as u16, 0)?;

        let identifier: [u8; MARKER_LENGTH] =
            data.value().try_into().map_err(|_| TpmError::Tpm("the marker has the wrong size".to_owned()))?;

        Ok(Some(identifier))
    }

    /// Writes the marker with the identifier of the installation.
    /// Fails with [`TpmError::MarkerExists`] when one is written.
    pub fn write_marker(&mut self, identifier: &[u8; MARKER_LENGTH]) -> Result<(), TpmError> {
        if self.marker_defined()? {
            return Err(TpmError::MarkerExists);
        }

        // Readable and writable with the authorization of the owner
        // of the TPM, which on Linux is empty.
        let attributes = NvIndexAttributesBuilder::new().with_owner_write(true).with_owner_read(true).build()?;

        let public = NvPublicBuilder::new()
            .with_nv_index(Self::marker_index()?)
            .with_index_name_algorithm(HashingAlgorithm::Sha256)
            .with_index_attributes(attributes)
            .with_data_area_size(MARKER_LENGTH)
            .build()?;

        let index = self.context.nv_define_space(Provision::Owner, None, public)?;

        self.context.nv_write(NvAuth::Owner, index, MaxNvBuffer::try_from(identifier.to_vec())?, 0)?;

        Ok(())
    }

    /// Removes the marker; nothing of the program stays in the TPM.
    /// Removing a marker that is not there is not an error.
    pub fn remove_marker(&mut self) -> Result<(), TpmError> {
        if !self.marker_defined()? {
            return Ok(());
        }

        let index = self.open_marker()?;

        self.context.nv_undefine_space(Provision::Owner, index)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests of `tpm.rs`, on the TPM emulator.

    use super::*;

    use crate::security::tpm::emulator::Emulator;
    use crate::security::tpm::test_support::TestTpm;

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

        let envelope = SealedEnvelope { public: vec![1, 2, 3], private: vec![4, 5] };

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
}
