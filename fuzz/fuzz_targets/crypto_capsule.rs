#![no_main]

use claw_core::hash::content_hash;
use claw_core::object::TypeTag;
use claw_core::types::CapsulePublic;
use claw_crypto::capsule::{build_capsule, verify_capsule};
use claw_crypto::encrypt::decrypt;
use claw_crypto::keypair::KeyPair;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let seed = array_32(data, 0);
    let key = array_32(data, 32);
    let private = data.get(64..).unwrap_or_default();
    let keypair = KeyPair::from_bytes(&seed).unwrap();
    let revision_id = content_hash(TypeTag::Revision, data);
    let public = CapsulePublic {
        agent_id: "fuzzer".to_string(),
        agent_version: None,
        toolchain_digest: None,
        env_fingerprint: None,
        evidence: vec![],
    };

    if let Ok(mut capsule) =
        build_capsule(&revision_id, public, Some(private), Some(&key), &keypair)
    {
        let public_key = keypair.public_key_bytes();
        let _ = verify_capsule(&capsule, &public_key);
        if let Some(encrypted) = capsule.encrypted_private.as_ref() {
            let _ = decrypt(&key, encrypted);
        }
        if let Some(encrypted) = capsule.encrypted_private.as_mut() {
            if !encrypted.is_empty() {
                encrypted[0] ^= 0x01;
            }
        }
        let _ = verify_capsule(&capsule, &public_key);
        if let Some(encrypted) = capsule.encrypted_private.as_ref() {
            let _ = decrypt(&key, encrypted);
        }
    }
});

fn array_32(data: &[u8], offset: usize) -> [u8; 32] {
    let mut out = [0u8; 32];
    if let Some(slice) = data.get(offset..offset.saturating_add(32)) {
        let len = slice.len().min(32);
        out[..len].copy_from_slice(&slice[..len]);
    }
    out
}
