use crate::{
    IcrcCompatibleAccount, ED25519_SIGNATURE_LENGTH, MAX_PUBKEY_BYTES, MINTING_ACCOUNT_PRINCIPAL,
};
use ed25519_dalek::ed25519::Error as DalekError;
use ed25519_dalek::pkcs8::spki::der::pem::LineEnding;
use ed25519_dalek::pkcs8::{EncodePrivateKey, EncodePublicKey};
use ed25519_dalek::{Digest, Sha512, Signature, SigningKey, VerifyingKey};
use hmac::{Hmac, Mac};
#[cfg(all(target_arch = "wasm32", feature = "ic"))]
#[allow(unused_imports)]
use ic_cdk::println;
use pkcs8::der::zeroize::Zeroizing;
use std::error::Error;
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
use std::io::Read;
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
use std::io::Write;
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
use std::path::PathBuf;

use crate::ED25519_SIGN_CONTEXT;

// See https://tools.ietf.org/html/rfc8410#section-10.3
const ED25519_OBJECT_IDENTIFIER: [u8; 3] = [43, 101, 112];
const ED25519_PEM_SIGNING_KEY_TAG: &str = "PRIVATE KEY";

#[repr(u8)]
#[derive(Debug)]
pub enum DccIdentity {
    Ed25519(Option<SigningKey>, VerifyingKey),
}

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

    pub fn new_signing_from_bytes(bytes: &[u8]) -> Result<Self, CryptoError> {
        let signing_key = Self::signing_key_from_bytes(bytes)?;
        DccIdentity::new_signing(&signing_key)
    }

    pub fn new_signing_from_der(der_key: &[u8]) -> Result<Self, CryptoError> {
        let signing_key = Self::signing_key_from_der(der_key)?;
        DccIdentity::new_signing(&signing_key)
    }

    pub fn new_signing_from_pem(pem_string: &str) -> Result<Self, CryptoError> {
        let signing_key = Self::signing_key_from_pem(pem_string)?;
        DccIdentity::new_signing(&signing_key)
    }

    pub fn new_verifying_from_pem(pem_string: &str) -> Result<Self, CryptoError> {
        let verifying_key = Self::verifying_key_from_pem(pem_string)?;
        DccIdentity::new_verifying(&verifying_key)
    }

    pub fn new_from_seed(seed: &[u8]) -> Result<Self, CryptoError> {
        let signing_key = Self::generate_ed25519_signing_key_from_seed(seed)?;
        DccIdentity::new_signing(&signing_key)
    }

    pub fn new_verifying_from_bytes(bytes: &[u8]) -> Result<Self, CryptoError> {
        if bytes.len() > MAX_PUBKEY_BYTES {
            return Err("Provided public key too long".into());
        }
        let bytes = slice_to_32_bytes_array(bytes)?;
        let verifying_key = VerifyingKey::from_bytes(bytes)?;
        DccIdentity::new_verifying(&verifying_key)
    }

    pub fn verifying_key(&self) -> &VerifyingKey {
        let DccIdentity::Ed25519(_, vk) = self;
        vk
    }

    pub fn to_ic_principal(&self) -> Result<candid::Principal, CryptoError> {
        let DccIdentity::Ed25519(_, vk) = self;
        let der = vk.to_public_key_der()?;
        Ok(candid::Principal::self_authenticating(der.as_bytes()))
    }

    pub fn as_icrc_compatible_account(&self) -> Result<IcrcCompatibleAccount, CryptoError> {
        Ok(IcrcCompatibleAccount::new(self.to_ic_principal()?, None))
    }

    /// Returns the public key as bytes.
    /// This is more universal than the IC principals, since an IC principal can be derived from the public key.
    pub fn to_bytes_verifying(&self) -> Vec<u8> {
        let DccIdentity::Ed25519(_, vk) = self;
        vk.to_bytes().to_vec()
    }

    /// Returns the public key in PEM format. This is more universal than the IC principals.
    pub fn as_uid_string(&self) -> Result<String, CryptoError> {
        self.verifying_key_as_pem_one_line()
    }

    pub fn is_minting_account(&self) -> Result<bool, CryptoError> {
        Ok(self.to_ic_principal()? == MINTING_ACCOUNT_PRINCIPAL)
    }

    pub fn to_der_signing(&self) -> Result<Vec<u8>, CryptoError> {
        let DccIdentity::Ed25519(sk, _) = self;
        let secret_key = sk
            .as_ref()
            .ok_or_else(|| CryptoError::Generic("no signing key available".to_string()))?;
        // According to https://tools.ietf.org/html/rfc8410#section-10.3
        let map_derp = |e: derp::Error| CryptoError::Generic(format!("DER encoding failed: {e}"));
        let mut key_bytes = vec![];
        let mut der = derp::Der::new(&mut key_bytes);
        der.octet_string(&secret_key.to_bytes()).map_err(map_derp)?;

        let mut encoded = vec![];
        der = derp::Der::new(&mut encoded);
        der.sequence(|der| {
            der.integer(&[0])?;
            der.sequence(|der| der.oid(&ED25519_OBJECT_IDENTIFIER))?;
            der.octet_string(&key_bytes)
        })
        .map_err(map_derp)?;
        Ok(encoded)
    }

    pub fn generate_ed25519_signing_key_from_seed(
        seed: &[u8],
    ) -> Result<SigningKey, CryptoError> {
        let mut mac = Hmac::<Sha512>::new_from_slice(b"ed25519 seed")
            .map_err(|e| CryptoError::Generic(format!("HMAC init failed: {e}")))?;
        mac.update(seed);
        let result = mac.finalize();
        let key_material = result.into_bytes();

        if key_material.len() < 32 {
            return Err(CryptoError::Generic(format!(
                "key material too short: {} < 32 bytes",
                key_material.len()
            )));
        }
        let mut seed_bytes = [0u8; 32];
        seed_bytes.copy_from_slice(&key_material[0..32]);
        Ok(SigningKey::from_bytes(&seed_bytes))
    }

    pub fn sign(&self, data: &[u8]) -> Result<Signature, CryptoError> {
        let DccIdentity::Ed25519(sk, _) = self;
        let signing_key = sk
            .as_ref()
            .ok_or_else(|| CryptoError::Generic("no signing key available".to_string()))?;
        let mut prehashed = Sha512::new();
        prehashed.update(data);
        Ok(signing_key.sign_prehashed(prehashed, Some(ED25519_SIGN_CONTEXT))?)
    }

    pub fn verify_bytes(&self, data: &[u8], signature_bytes: &[u8]) -> Result<(), CryptoError> {
        if signature_bytes.len() != ED25519_SIGNATURE_LENGTH {
            return Err("Invalid signature".into());
        }
        let signature = Signature::from_bytes(slice_to_64_bytes_array(signature_bytes)?);
        self.verify(data, &signature)
    }

    pub fn verify(&self, data: &[u8], signature: &Signature) -> Result<(), CryptoError> {
        let DccIdentity::Ed25519(_, vk) = self;
        let mut prehashed = Sha512::new();
        prehashed.update(data);
        Ok(vk.verify_prehashed(prehashed, Some(ED25519_SIGN_CONTEXT), signature)?)
    }

    pub fn verifying_key_as_pem(&self) -> Result<String, CryptoError> {
        let DccIdentity::Ed25519(_, vk) = self;
        Ok(vk.to_public_key_pem(LineEnding::LF)?)
    }

    pub fn verifying_key_as_pem_one_line(&self) -> Result<String, CryptoError> {
        let pem = self.verifying_key_as_pem()?;
        let body = pem
            .trim()
            .strip_prefix("-----BEGIN PUBLIC KEY-----")
            .ok_or_else(|| CryptoError::Generic("PEM missing BEGIN header".to_string()))?
            .strip_suffix("-----END PUBLIC KEY-----")
            .ok_or_else(|| CryptoError::Generic("PEM missing END header".to_string()))?
            .trim()
            .replace('\n', "");
        Ok(body)
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    pub fn write_verifying_key_to_pem_file(&self, file_path: &PathBuf) -> Result<(), CryptoError> {
        let pem_string = self.verifying_key_as_pem()?;
        let parent = file_path.parent().ok_or_else(|| {
            CryptoError::Generic(format!("file path has no parent: {}", file_path.display()))
        })?;
        fs_err::create_dir_all(parent)?;
        let mut file = fs_err::File::create(file_path)?;
        file.write_all(pem_string.as_bytes())?;
        Ok(())
    }

    pub fn signing_key_as_ic_agent_pem_string(
        &self,
    ) -> Result<Zeroizing<String>, CryptoError> {
        let DccIdentity::Ed25519(sk, _) = self;
        let signing_key = sk
            .as_ref()
            .ok_or_else(|| CryptoError::Generic("no signing key available".to_string()))?;
        Ok(signing_key.to_pkcs8_pem(LineEnding::LF)?)
    }

    pub fn signing_key_as_pem_string(&self) -> Result<String, CryptoError> {
        let contents = self.to_der_signing()?;
        let pem_obj = pem::Pem::new(ED25519_PEM_SIGNING_KEY_TAG, contents);
        Ok(pem::encode(&pem_obj))
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    pub fn write_signing_key_to_pem_file(&self, file_path: &PathBuf) -> Result<(), CryptoError> {
        let pem_string = self.signing_key_as_pem_string()?;
        let parent = file_path.parent().ok_or_else(|| {
            CryptoError::Generic(format!("file path has no parent: {}", file_path.display()))
        })?;
        fs_err::create_dir_all(parent)?;
        let mut file = fs_err::File::create(file_path)?;
        file.write_all(pem_string.as_bytes())?;
        Ok(())
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    pub fn identities_dir() -> Result<PathBuf, CryptoError> {
        let home = dirs::home_dir()
            .ok_or_else(|| CryptoError::Generic("home directory not found".to_string()))?;
        Ok(home.join(".dcc").join("identity"))
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    pub fn save_to_dir(&self, identity: &str) -> Result<(), CryptoError> {
        let identity_dir = Self::identities_dir()?.join(identity);

        let public_pem_file_path = identity_dir.join("public.pem");
        if public_pem_file_path.exists() {
            return Err(format!(
                "Refusing to overwrite existing public key file at {}",
                public_pem_file_path.display()
            )
            .into());
        }
        self.write_verifying_key_to_pem_file(&public_pem_file_path)?;

        let DccIdentity::Ed25519(sk, _) = self;
        if sk.is_some() {
            self.write_signing_key_to_pem_file(&identity_dir.join("private.pem"))?;
        }

        Ok(())
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    fn read_signing_key_from_pem_file(file_path: &PathBuf) -> Result<SigningKey, CryptoError> {
        let mut file = fs_err::File::open(file_path)?;
        let mut pem_string = String::new();
        file.read_to_string(&mut pem_string)?;

        Self::signing_key_from_pem(&pem_string)
    }

    pub fn signing_key_from_pem(pem_string: &str) -> Result<SigningKey, CryptoError> {
        let pem = pem::parse(pem_string)?;
        let key = ed25519_dalek::pkcs8::DecodePrivateKey::from_pkcs8_der(pem.contents())?;
        Ok(key)
    }

    pub fn signing_key_from_der(der_key: &[u8]) -> Result<SigningKey, CryptoError> {
        let key = ed25519_dalek::pkcs8::DecodePrivateKey::from_pkcs8_der(der_key)?;
        Ok(key)
    }

    pub fn signing_key_from_bytes(bytes: &[u8]) -> Result<SigningKey, CryptoError> {
        if bytes.len() > MAX_PUBKEY_BYTES {
            return Err("Provided public key too long".into());
        }
        let bytes = slice_to_32_bytes_array(bytes)?;
        let key = SigningKey::from_bytes(bytes);
        Ok(key)
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    fn read_verifying_key_from_pem_file(file_path: &PathBuf) -> Result<VerifyingKey, CryptoError> {
        let mut file = fs_err::File::open(file_path)?;
        let mut pem_string = String::new();
        file.read_to_string(&mut pem_string)?;

        Self::verifying_key_from_pem(&pem_string)
    }

    pub fn verifying_key_from_pem(pem_string: &str) -> Result<VerifyingKey, CryptoError> {
        let pem_string = if pem_string.starts_with("-----BEGIN PUBLIC KEY-----") {
            pem_string
        } else {
            &format!(
                "-----BEGIN PUBLIC KEY-----\n{}\n-----END PUBLIC KEY-----",
                pem_string
            )
        };
        let key = ed25519_dalek::pkcs8::DecodePublicKey::from_public_key_pem(pem_string)?;
        Ok(key)
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    pub fn load_from_dir(identity_dir: &PathBuf) -> Result<Self, CryptoError> {
        let identity_dir = if identity_dir.is_absolute() {
            identity_dir.to_path_buf()
        } else {
            Self::identities_dir()?.join(identity_dir)
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

    pub fn display_type(&self) -> &str {
        let DccIdentity::Ed25519(sk, _) = self;
        match sk {
            Some(_) => "[Ed25519 signing]",
            None => "[Ed25519 verifying]",
        }
    }

    pub fn display_verifying_key_as_ic_identity(&self) -> String {
        match self.to_ic_principal() {
            Ok(principal) => principal.to_string(),
            Err(e) => format!("[crypto error: {e}]"),
        }
    }

    pub fn display_verifying_key_as_pem_one_line(&self) -> String {
        match self.verifying_key_as_pem_one_line() {
            Ok(pem) => pem,
            Err(e) => format!("[crypto error: {e}]"),
        }
    }

    pub fn display_as_pem_one_line(&self) -> String {
        format!(
            "{} {}",
            self.display_type(),
            self.display_verifying_key_as_pem_one_line()
        )
    }

    pub fn display_as_ic_and_pem_one_line(&self) -> String {
        format!(
            "{} IC identity {} PEM {}",
            self.display_type(),
            self.display_verifying_key_as_ic_identity(),
            self.display_verifying_key_as_pem_one_line()
        )
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

impl From<CryptoError> for String {
    fn from(error: CryptoError) -> Self {
        error.to_string()
    }
}

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

impl From<&str> for CryptoError {
    fn from(error: &str) -> Self {
        CryptoError::Generic(error.to_string())
    }
}

impl std::fmt::Display for DccIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{} {}",
            self.display_type(),
            self.display_verifying_key_as_ic_identity()
        )
    }
}

pub fn slice_to_32_bytes_array(slice: &[u8]) -> Result<&[u8; 32], String> {
    slice
        .try_into()
        .map_err(|_| format!("slice length is {} instead of 32 bytes", slice.len()))
}

pub fn slice_to_64_bytes_array(slice: &[u8]) -> Result<&[u8; 64], String> {
    slice
        .try_into()
        .map_err(|_| format!("slice length is {} instead of 64 bytes", slice.len()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_signing_identity() -> DccIdentity {
        let seed = [0u8; 32];
        let signing_key = DccIdentity::generate_ed25519_signing_key_from_seed(&seed).unwrap();
        DccIdentity::new_signing(&signing_key).unwrap()
    }

    fn test_verifying_identity() -> DccIdentity {
        let seed = [0u8; 32];
        let signing_key = DccIdentity::generate_ed25519_signing_key_from_seed(&seed).unwrap();
        DccIdentity::new_verifying(&signing_key.verifying_key()).unwrap()
    }

    #[test]
    fn test_new_signing() {
        let id = test_signing_identity();
        let DccIdentity::Ed25519(sk, _) = &id;
        assert!(sk.is_some());
    }

    #[test]
    fn test_new_verifying() {
        let id = test_verifying_identity();
        let DccIdentity::Ed25519(sk, _) = &id;
        assert!(sk.is_none());
    }

    #[test]
    fn test_new_from_seed() {
        let dcc_identity = DccIdentity::new_from_seed(&[0u8; 32]).unwrap();
        let DccIdentity::Ed25519(sk, _) = &dcc_identity;
        assert!(sk.is_some());
    }

    #[test]
    fn test_new_verifying_from_bytes() {
        let seed = [0u8; 32];
        let signing_key = DccIdentity::generate_ed25519_signing_key_from_seed(&seed).unwrap();
        let verifying_key_bytes = signing_key.verifying_key().to_bytes();
        let id = DccIdentity::new_verifying_from_bytes(&verifying_key_bytes).unwrap();
        let DccIdentity::Ed25519(sk, _) = &id;
        assert!(sk.is_none());
    }

    #[test]
    fn test_verifying_key() {
        let seed = [0u8; 32];
        let signing_key = DccIdentity::generate_ed25519_signing_key_from_seed(&seed).unwrap();
        let dcc_identity = DccIdentity::new_signing(&signing_key).unwrap();
        assert_eq!(
            dcc_identity.verifying_key().to_bytes(),
            signing_key.verifying_key().to_bytes()
        );
    }

    #[test]
    fn test_to_ic_principal() {
        let id = test_signing_identity();
        let principal = id.to_ic_principal().unwrap();
        assert!(!principal.to_text().is_empty());
    }

    #[test]
    fn test_to_account() {
        let id = test_signing_identity();
        assert_eq!(
            id.as_icrc_compatible_account().unwrap().owner.to_text(),
            "tuke6-qjtdo-jtp67-maudl-vlprb-szzw2-fvlmx-a5i3v-pqqbt-tmn3q-eae"
        );
    }

    #[test]
    fn test_to_bytes_verifying() {
        let id = test_signing_identity();
        assert_eq!(id.to_bytes_verifying().len(), 32);
    }

    #[test]
    fn test_sign_and_verify() {
        let id = test_signing_identity();
        let message: &[u8] = b"Test Message";
        let signature = id.sign(message).unwrap();
        assert!(id.verify(message, &signature).is_ok());
    }

    #[test]
    fn test_sign_fails_without_signing_key() {
        let id = test_verifying_identity();
        let result = id.sign(b"data");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no signing key"));
    }

    #[test]
    fn test_to_der_signing_fails_without_signing_key() {
        let id = test_verifying_identity();
        let result = id.to_der_signing();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no signing key"));
    }

    #[test]
    fn test_signing_key_as_ic_agent_pem_fails_without_signing_key() {
        let id = test_verifying_identity();
        let result = id.signing_key_as_ic_agent_pem_string();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no signing key"));
    }

    #[test]
    fn test_verifying_key_as_pem() {
        let id = test_signing_identity();
        let pem = id.verifying_key_as_pem().unwrap();
        assert!(pem.starts_with("-----BEGIN PUBLIC KEY-----"));
    }

    #[test]
    fn test_verifying_key_as_pem_one_line() {
        let id = test_signing_identity();
        let pem_one_line = id.verifying_key_as_pem_one_line().unwrap();
        assert!(!pem_one_line.contains('\n'));
    }

    #[test]
    fn test_signing_key_as_pem_string() {
        let id = test_signing_identity();
        let pem_string = id.signing_key_as_pem_string();
        assert!(pem_string.is_ok());
    }

    #[test]
    fn test_slice_to_32_bytes_wrong_length() {
        assert!(slice_to_32_bytes_array(&[0u8; 31]).is_err());
        assert!(slice_to_32_bytes_array(&[0u8; 33]).is_err());
        assert!(slice_to_32_bytes_array(&[0u8; 32]).is_ok());
    }

    #[test]
    fn test_slice_to_64_bytes_wrong_length() {
        assert!(slice_to_64_bytes_array(&[0u8; 63]).is_err());
        assert!(slice_to_64_bytes_array(&[0u8; 65]).is_err());
        assert!(slice_to_64_bytes_array(&[0u8; 64]).is_ok());
    }
}
