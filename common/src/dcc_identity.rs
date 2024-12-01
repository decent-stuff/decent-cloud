use ed25519_dalek::ed25519::Error as DalekError;
use ed25519_dalek::pkcs8::spki::der::pem::LineEnding;
#[cfg(target_arch = "wasm32")]
#[allow(unused_imports)]
use ic_cdk::println;
use pkcs8::der::zeroize::Zeroizing;

use crate::{IcrcCompatibleAccount, MINTING_ACCOUNT_PRINCIPAL};
use ed25519_dalek::pkcs8::{EncodePrivateKey, EncodePublicKey};
use ed25519_dalek::{Digest, Sha512, Signature, SigningKey, VerifyingKey};
use hmac::{Hmac, Mac};
use std::error::Error;
#[cfg(target_arch = "x86_64")]
use std::io::Read;
#[cfg(target_arch = "x86_64")]
use std::io::Write;
#[cfg(target_arch = "x86_64")]
use std::path::PathBuf;

use crate::ED25519_SIGN_CONTEXT;

// See https://tools.ietf.org/html/rfc8410#section-10.3
const ED25519_OBJECT_IDENTIFIER: [u8; 3] = [43, 101, 112];
const ED25519_PEM_SIGNING_KEY_TAG: &str = "PRIVATE KEY";
// const ED25519_PEM_VERIFYING_KEY_TAG: &str = "PUBLIC KEY";

// const SECP256K1_OBJECT_IDENTIFIER: [u8; 5] = [43, 129, 4, 0, 10];
// const SECP256K1_PEM_SIGNING_KEY_TAG: &str = "EC PRIVATE KEY";
// const SECP256K1_PEM_VERIFYING_KEY_TAG: &str = "PUBLIC KEY";

#[repr(u8)]
#[derive(Debug)]
pub enum DccIdentity {
    Ed25519(Option<SigningKey>, VerifyingKey),
}

// #[derive(Debug)]
// pub struct DccIdentity {
//     ed25519_signing_key: Option<SigningKey>,
//     ed25519_verifying_key: VerifyingKey,
// }

impl DccIdentity {
    pub fn new_signing(ed25519_signing_key: &SigningKey) -> Result<Self, CryptoError> {
        Ok(DccIdentity::Ed25519(
            Some(ed25519_signing_key.clone()),
            ed25519_signing_key.verifying_key(),
        ))
    }

    pub fn new_verifying(ed25519_verifying_key: &VerifyingKey) -> Result<Self, CryptoError> {
        Ok(DccIdentity::Ed25519(None, *ed25519_verifying_key))
    }

    pub fn new_from_seed(seed: &[u8]) -> Result<Self, CryptoError> {
        let signing_key = Self::generate_ed25519_signing_key_from_seed(seed);
        DccIdentity::new_signing(&signing_key)
    }

    pub fn new_verifying_from_bytes(bytes: &[u8]) -> Result<Self, CryptoError> {
        let bytes = slice_to_32_bytes_array(bytes)?;
        let verifying_key = VerifyingKey::from_bytes(bytes)?;
        DccIdentity::new_verifying(&verifying_key)
    }

    pub fn verifying_key(&self) -> &VerifyingKey {
        match self {
            DccIdentity::Ed25519(_, verifying_key) => verifying_key,
        }
    }

    pub fn to_ic_principal(&self) -> candid::Principal {
        match self {
            DccIdentity::Ed25519(_, verifying_key) => {
                let der = verifying_key
                    .to_public_key_der()
                    .expect("failed to encode public key to der");
                candid::Principal::self_authenticating(der.as_bytes())
            }
        }
    }

    pub fn as_icrc_compatible_account(&self) -> IcrcCompatibleAccount {
        IcrcCompatibleAccount::new(self.to_ic_principal(), None)
    }

    /// Returns the public key as bytes.
    /// This is more universal than the IC principals, since an IC principal can be derived from the public key.
    pub fn to_bytes_verifying(&self) -> Vec<u8> {
        match self {
            DccIdentity::Ed25519(_, verifying_key) => verifying_key.to_bytes().to_vec(),
        }
    }

    /// Returns the public key in PEM format. This is more universal than the IC principals.
    pub fn as_uid_string(&self) -> String {
        self.verifying_key_as_pem_one_line().to_owned()
    }

    pub fn is_minting_account(&self) -> bool {
        self.to_ic_principal() == MINTING_ACCOUNT_PRINCIPAL
    }

    pub fn to_der_signing(&self) -> Vec<u8> {
        match self {
            DccIdentity::Ed25519(Some(secret_key), _verifying_key) => {
                // According to https://tools.ietf.org/html/rfc8410#section-10.3
                let mut key_bytes = vec![];
                let mut der = derp::Der::new(&mut key_bytes);
                der.octet_string(&secret_key.to_bytes())
                    .expect("failed to serialize the signing key");

                let mut encoded = vec![];
                der = derp::Der::new(&mut encoded);
                der.sequence(|der| {
                    der.integer(&[0])?;
                    der.sequence(|der| der.oid(&ED25519_OBJECT_IDENTIFIER))?;
                    der.octet_string(&key_bytes)
                })
                .expect("failed to prepare der formatted signing key");
                encoded
            }
            _ => panic!("Invalid type of DccIdentity"),
        }
    }

    pub fn generate_ed25519_signing_key_from_seed(seed: &[u8]) -> SigningKey {
        let mut mac = Hmac::<Sha512>::new_from_slice(b"ed25519 seed").unwrap();
        mac.update(seed);
        let result = mac.finalize();
        let key_material = result.into_bytes();

        if key_material.len() < 32 {
            panic!(
                "Key material should be at least 32 bytes, got {}",
                key_material.len()
            );
        }
        let mut seed_bytes = [0u8; 32];
        seed_bytes.copy_from_slice(&key_material[0..32]);
        SigningKey::from_bytes(&seed_bytes)
    }

    pub fn sign(&self, data: &[u8]) -> Result<Signature, CryptoError> {
        match self {
            DccIdentity::Ed25519(Some(signing_key), _) => {
                let mut prehashed = Sha512::new();
                prehashed.update(data);
                Ok(signing_key.sign_prehashed(prehashed, Some(ED25519_SIGN_CONTEXT))?)
            }
            _ => panic!("Invalid type of DccIdentity"),
        }
    }

    pub fn verify_bytes(&self, data: &[u8], signature_bytes: &[u8]) -> Result<(), CryptoError> {
        let signature = Signature::from_bytes(slice_to_64_bytes_array(signature_bytes)?);
        self.verify(data, &signature)
    }

    pub fn verify(&self, data: &[u8], signature: &Signature) -> Result<(), CryptoError> {
        match self {
            DccIdentity::Ed25519(_, verifying_key) => {
                let mut prehashed = Sha512::new();
                prehashed.update(data);
                Ok(verifying_key.verify_prehashed(
                    prehashed,
                    Some(ED25519_SIGN_CONTEXT),
                    signature,
                )?)
            }
        }
    }

    pub fn verifying_key_as_pem(&self) -> String {
        match self {
            DccIdentity::Ed25519(_, verifying_key) => verifying_key
                .to_public_key_pem(LineEnding::LF)
                .expect("pem encode failed"),
        }
    }

    pub fn verifying_key_as_pem_one_line(&self) -> String {
        self.verifying_key_as_pem()
            .trim()
            .strip_prefix("-----BEGIN PUBLIC KEY-----")
            .expect("Strip prefix strip failed")
            .strip_suffix("-----END PUBLIC KEY-----")
            .expect("Strip suffix strip failed")
            .trim()
            .replace('\n', "")
    }

    #[cfg(target_arch = "x86_64")]
    pub fn write_verifying_key_to_pem_file(&self, file_path: &PathBuf) -> Result<(), CryptoError> {
        let pem_string = self.verifying_key_as_pem();

        fs_err::create_dir_all(file_path.parent().expect("file_path has no parent"))?;
        let mut file = fs_err::File::create(file_path)?;
        file.write_all(pem_string.as_bytes())?;

        Ok(())
    }

    pub fn signing_key_as_ic_agent_pem_string(&self) -> Option<Zeroizing<String>> {
        match self {
            DccIdentity::Ed25519(Some(signing_key), _) => Some(
                signing_key
                    .to_pkcs8_pem(LineEnding::LF)
                    .expect("pem encode failed"),
            ),
            _ => None,
        }
    }

    pub fn signing_key_as_pem_string(&self) -> Result<String, CryptoError> {
        let contents = self.to_der_signing();
        let pem_obj = pem::Pem::new(ED25519_PEM_SIGNING_KEY_TAG, contents);
        Ok(pem::encode(&pem_obj))
    }

    #[cfg(target_arch = "x86_64")]
    pub fn write_signing_key_to_pem_file(&self, file_path: &PathBuf) -> Result<(), CryptoError> {
        let pem_string = self.signing_key_as_pem_string()?;

        fs_err::create_dir_all(file_path.parent().expect("file_path has no parent"))?;
        let mut file = fs_err::File::create(file_path)?;
        file.write_all(pem_string.as_bytes())?;

        Ok(())
    }

    #[cfg(target_arch = "x86_64")]
    pub fn identities_dir() -> PathBuf {
        dirs::home_dir()
            .expect("Failed to find home directory")
            .join(".dcc")
            .join("identity")
    }

    #[cfg(target_arch = "x86_64")]
    pub fn save_to_dir(&self, identity: &str) -> Result<(), CryptoError> {
        let identity_dir = Self::identities_dir().join(identity);

        let public_pem_file_path = identity_dir.join("public.pem");
        self.write_verifying_key_to_pem_file(&public_pem_file_path)?;

        match self {
            DccIdentity::Ed25519(Some(_), _) => {
                self.write_signing_key_to_pem_file(&identity_dir.join("private.pem"))?;
            }
            DccIdentity::Ed25519(None, _) => {}
        }

        Ok(())
    }

    #[cfg(target_arch = "x86_64")]
    fn read_signing_key_from_pem_file(file_path: &PathBuf) -> Result<SigningKey, CryptoError> {
        let mut file = fs_err::File::open(file_path)?;
        let mut pem_string = String::new();
        file.read_to_string(&mut pem_string)?;
        let pem = pem::parse(pem_string)?;

        // let secret_key = Self::from_der(&pem.contents())?;
        let key = ed25519_dalek::pkcs8::DecodePrivateKey::from_pkcs8_der(pem.contents())?;
        // let key = ed25519_dalek::pkcs8::DecodePrivateKey::from_pkcs8_pem(&pem_string)?;
        Ok(key)
    }

    #[cfg(target_arch = "x86_64")]
    fn read_verifying_key_from_pem_file(file_path: &PathBuf) -> Result<VerifyingKey, CryptoError> {
        let mut file = fs_err::File::open(file_path)?;
        let mut pem_string = String::new();
        file.read_to_string(&mut pem_string)?;

        let key = ed25519_dalek::pkcs8::DecodePublicKey::from_public_key_pem(&pem_string)?;
        Ok(key)
    }

    #[cfg(target_arch = "x86_64")]
    pub fn load_from_dir(identity_dir: &PathBuf) -> Result<Self, CryptoError> {
        let identity_dir = if identity_dir.is_absolute() {
            identity_dir.to_path_buf()
        } else {
            Self::identities_dir().join(identity_dir)
        };
        let private_pem_file_path = identity_dir.join("private.pem");
        if private_pem_file_path.exists() {
            let signing_key = Self::read_signing_key_from_pem_file(&private_pem_file_path)?;
            Self::new_signing(&signing_key)
        } else {
            let public_pem_file_path = identity_dir.join("public.pem");

            let verifying_key = Self::read_verifying_key_from_pem_file(&public_pem_file_path)?;

            Self::new_verifying(&verifying_key)
        }
    }
}

#[derive(Debug)]
pub enum CryptoError {
    DalekError(DalekError),
    Pkcs8Error(pkcs8::Error),
    PemError(pem::PemError),
    IoError(std::io::Error),
    Generic(String),
}

impl std::fmt::Display for CryptoError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            CryptoError::DalekError(e) => e.fmt(f),
            CryptoError::Pkcs8Error(e) => e.fmt(f),
            CryptoError::PemError(e) => e.fmt(f),
            CryptoError::IoError(e) => e.fmt(f),
            CryptoError::Generic(e) => e.fmt(f),
        }
    }
}

impl Error for CryptoError {}

impl From<DalekError> for CryptoError {
    fn from(error: DalekError) -> Self {
        CryptoError::DalekError(error)
    }
}

impl From<pkcs8::Error> for CryptoError {
    fn from(error: pkcs8::Error) -> Self {
        CryptoError::Pkcs8Error(error)
    }
}

impl From<pkcs8::spki::Error> for CryptoError {
    fn from(error: pkcs8::spki::Error) -> Self {
        CryptoError::Pkcs8Error(error.into())
    }
}

impl From<pem::PemError> for CryptoError {
    fn from(error: pem::PemError) -> Self {
        CryptoError::PemError(error)
    }
}

impl From<String> for CryptoError {
    fn from(error: String) -> Self {
        CryptoError::Generic(error)
    }
}

impl From<std::io::Error> for CryptoError {
    fn from(error: std::io::Error) -> Self {
        CryptoError::IoError(error)
    }
}

impl std::fmt::Display for DccIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            DccIdentity::Ed25519(None, _) => write!(f, "[Ed25519 verifying] ")?,
            DccIdentity::Ed25519(Some(_), _) => write!(f, "[Ed25519 signing] ")?,
        }
        write!(f, "{}", self.to_ic_principal())
    }
}

pub fn slice_to_32_bytes_array(slice: &[u8]) -> Result<&[u8; 32], String> {
    if slice.len() == 32 {
        Ok(slice.try_into().expect("slice with incorrect length"))
    } else {
        Err(format!(
            "slice length is {} instead of 32 bytes",
            slice.len()
        ))
    }
}

pub fn slice_to_64_bytes_array(slice: &[u8]) -> Result<&[u8; 64], String> {
    if slice.len() == 64 {
        Ok(slice.try_into().expect("slice with incorrect length"))
    } else {
        Err(format!(
            "slice length is {} instead of 64 bytes",
            slice.len()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_signing() {
        let seed = [0u8; 32];
        let signing_key = DccIdentity::generate_ed25519_signing_key_from_seed(&seed);
        let dcc_identity = DccIdentity::new_signing(&signing_key).unwrap();
        match dcc_identity {
            DccIdentity::Ed25519(Some(_), _) => {}
            _ => panic!("Invalid type of DccIdentity"),
        }
    }

    #[test]
    fn test_new_verifying() {
        let seed = [0u8; 32];
        let signing_key = DccIdentity::generate_ed25519_signing_key_from_seed(&seed);
        let verifying_key = signing_key.verifying_key();
        let dcc_identity = DccIdentity::new_verifying(&verifying_key).unwrap();
        match dcc_identity {
            DccIdentity::Ed25519(None, _) => {}
            _ => panic!("Invalid type of DccIdentity"),
        }
    }

    #[test]
    fn test_new_from_seed() {
        let seed = [0u8; 32];
        let dcc_identity = DccIdentity::new_from_seed(&seed).unwrap();
        match dcc_identity {
            DccIdentity::Ed25519(Some(_), _) => {}
            _ => panic!("Invalid type of DccIdentity"),
        }
    }

    #[test]
    fn test_new_verifying_from_bytes() {
        let seed = [0u8; 32];
        let signing_key = DccIdentity::generate_ed25519_signing_key_from_seed(&seed);
        let verifying_key_bytes = signing_key.verifying_key().to_bytes();
        let dcc_identity = DccIdentity::new_verifying_from_bytes(&verifying_key_bytes).unwrap();
        match dcc_identity {
            DccIdentity::Ed25519(None, _) => {}
            _ => panic!("Invalid type of DccIdentity"),
        }
    }

    #[test]
    fn test_verifying_key() {
        let seed = [0u8; 32];
        let signing_key = DccIdentity::generate_ed25519_signing_key_from_seed(&seed);
        let dcc_identity = DccIdentity::new_signing(&signing_key).unwrap();
        assert_eq!(
            dcc_identity.verifying_key().to_bytes(),
            signing_key.verifying_key().to_bytes()
        );
    }

    #[test]
    fn test_to_ic_principal() {
        let seed = [0u8; 32];
        let signing_key = DccIdentity::generate_ed25519_signing_key_from_seed(&seed);
        let dcc_identity = DccIdentity::new_signing(&signing_key).unwrap();
        let principal = dcc_identity.to_ic_principal();
        assert!(!principal.to_text().is_empty());
    }

    #[test]
    fn test_to_account() {
        let seed = [0u8; 32];
        let signing_key = DccIdentity::generate_ed25519_signing_key_from_seed(&seed);
        let dcc_identity = DccIdentity::new_signing(&signing_key).unwrap();
        assert_eq!(
            dcc_identity.as_icrc_compatible_account().owner.to_text(),
            "tuke6-qjtdo-jtp67-maudl-vlprb-szzw2-fvlmx-a5i3v-pqqbt-tmn3q-eae"
        );
    }

    #[test]
    fn test_to_bytes_verifying() {
        let seed = [0u8; 32];
        let signing_key = DccIdentity::generate_ed25519_signing_key_from_seed(&seed);
        let dcc_identity = DccIdentity::new_signing(&signing_key).unwrap();
        let verifying_bytes = dcc_identity.to_bytes_verifying();
        assert_eq!(verifying_bytes.len(), 32);
    }

    #[test]
    fn test_sign_and_verify() {
        let seed = [0u8; 32];
        let signing_key = DccIdentity::generate_ed25519_signing_key_from_seed(&seed);
        let dcc_identity = DccIdentity::new_signing(&signing_key).unwrap();

        let message: &[u8] = b"Test Message";
        let signature = dcc_identity.sign(message).unwrap();
        assert!(dcc_identity.verify(message, &signature).is_ok());
    }

    #[test]
    fn test_verifying_key_as_pem() {
        let seed = [0u8; 32];
        let signing_key = DccIdentity::generate_ed25519_signing_key_from_seed(&seed);
        let dcc_identity = DccIdentity::new_signing(&signing_key).unwrap();
        let pem = dcc_identity.verifying_key_as_pem();
        assert!(pem.starts_with("-----BEGIN PUBLIC KEY-----"));
    }

    #[test]
    fn test_verifying_key_as_pem_one_line() {
        let seed = [0u8; 32];
        let signing_key = DccIdentity::generate_ed25519_signing_key_from_seed(&seed);
        let dcc_identity = DccIdentity::new_signing(&signing_key).unwrap();
        let pem_one_line = dcc_identity.verifying_key_as_pem_one_line();
        assert!(!pem_one_line.contains('\n'));
    }

    #[test]
    fn test_signing_key_as_pem_string() {
        let seed = [0u8; 32];
        let signing_key = DccIdentity::generate_ed25519_signing_key_from_seed(&seed);
        let dcc_identity = DccIdentity::new_signing(&signing_key).unwrap();
        let pem_string = dcc_identity.signing_key_as_pem_string();
        assert!(pem_string.is_ok());
    }
}
