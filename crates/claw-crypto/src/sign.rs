use ed25519_dalek::Signer;

use crate::keypair::KeyPair;

/// Detached Ed25519 signature plus signer public-key bytes.
pub struct Signature {
    /// Public key bytes of the signer.
    pub signer_id: Vec<u8>,
    /// Raw Ed25519 signature bytes.
    pub signature: Vec<u8>,
}

/// Signs arbitrary bytes with a Claw keypair.
pub fn sign(keypair: &KeyPair, data: &[u8]) -> Signature {
    let sig = keypair.signing_key().sign(data);
    Signature {
        signer_id: keypair.public_key_bytes().to_vec(),
        signature: sig.to_bytes().to_vec(),
    }
}
