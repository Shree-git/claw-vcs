use ed25519_dalek::{SigningKey, VerifyingKey};
use rand::rngs::OsRng;

use crate::CryptoError;

/// Ed25519 keypair used to sign capsules and related provenance claims.
pub struct KeyPair {
    signing_key: SigningKey,
}

impl KeyPair {
    /// Generates a new Ed25519 keypair with operating-system randomness.
    pub fn generate() -> Self {
        let signing_key = SigningKey::generate(&mut OsRng);
        Self { signing_key }
    }

    /// Constructs a keypair from a 32-byte Ed25519 secret key seed.
    pub fn from_bytes(bytes: &[u8; 32]) -> Result<Self, CryptoError> {
        let signing_key = SigningKey::from_bytes(bytes);
        Ok(Self { signing_key })
    }

    /// Returns the underlying Ed25519 signing key.
    pub fn signing_key(&self) -> &SigningKey {
        &self.signing_key
    }

    /// Returns the corresponding Ed25519 verifying key.
    pub fn verifying_key(&self) -> VerifyingKey {
        self.signing_key.verifying_key()
    }

    /// Serializes the secret key seed.
    pub fn to_bytes(&self) -> [u8; 32] {
        self.signing_key.to_bytes()
    }

    /// Returns the public verifying key bytes.
    pub fn public_key_bytes(&self) -> [u8; 32] {
        self.verifying_key().to_bytes()
    }

    /// Writes the secret key seed to `path`.
    pub fn save_to_file(&self, path: &std::path::Path) -> Result<(), CryptoError> {
        std::fs::write(path, self.to_bytes())?;
        Ok(())
    }

    /// Loads a keypair from a file containing exactly 32 secret-key bytes.
    pub fn load_from_file(path: &std::path::Path) -> Result<Self, CryptoError> {
        let bytes = std::fs::read(path)?;
        let arr: [u8; 32] = bytes
            .try_into()
            .map_err(|_| CryptoError::InvalidKey("expected 32 bytes".into()))?;
        Self::from_bytes(&arr)
    }
}
