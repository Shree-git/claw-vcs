use claw_core::hash::content_hash;
use claw_core::object::TypeTag;
use claw_core::types::{CapsulePublic, Evidence};
use claw_crypto::capsule::{build_capsule, verify_capsule};
use claw_crypto::encrypt::{decrypt, encrypt};
use claw_crypto::keypair::KeyPair;
use proptest::prelude::*;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct CryptoVector {
    name: String,
    signing_seed_hex: String,
    encryption_key_hex: String,
    revision_payload_hex: String,
    private_hex: String,
    agent_id: String,
}

#[test]
fn encrypted_capsule_vector_detects_public_private_and_signature_tampering() {
    let vector: CryptoVector = serde_json::from_str(include_str!(
        "../../../tests/vectors/crypto_capsule_vector.json"
    ))
    .unwrap();
    let signing_seed = hex_array_32(&vector.signing_seed_hex);
    let encryption_key = hex_array_32(&vector.encryption_key_hex);
    let private_data = decode_hex(&vector.private_hex);
    let revision_payload = decode_hex(&vector.revision_payload_hex);
    let revision_id = content_hash(TypeTag::Revision, &revision_payload);
    let keypair = KeyPair::from_bytes(&signing_seed).unwrap();
    let public_key = keypair.public_key_bytes();

    let capsule = build_capsule(
        &revision_id,
        public_fields(&vector.agent_id),
        Some(&private_data),
        Some(&encryption_key),
        &keypair,
    )
    .unwrap();

    assert!(
        verify_capsule(&capsule, &public_key).unwrap(),
        "{} should verify before mutation",
        vector.name
    );
    assert_eq!(
        decrypt(&encryption_key, capsule.encrypted_private.as_ref().unwrap()).unwrap(),
        private_data
    );

    let mut public_tampered = capsule.clone();
    public_tampered.public_fields.agent_version = Some("tampered".to_string());
    assert!(!verify_capsule(&public_tampered, &public_key).unwrap());

    let mut encrypted_tampered = capsule.clone();
    {
        let encrypted = encrypted_tampered.encrypted_private.as_mut().unwrap();
        let last = encrypted.len() - 1;
        encrypted[last] ^= 0x01;
    }
    assert!(!verify_capsule(&encrypted_tampered, &public_key).unwrap());
    assert!(decrypt(
        &encryption_key,
        encrypted_tampered.encrypted_private.as_ref().unwrap()
    )
    .is_err());

    let mut signature_tampered = capsule;
    signature_tampered.signatures[0].signature[0] ^= 0x01;
    assert!(!verify_capsule(&signature_tampered, &public_key).unwrap());
}

proptest! {
    #[test]
    fn encryption_roundtrips_and_rejects_ciphertext_tamper(
        key in any::<[u8; 32]>(),
        plaintext in prop::collection::vec(any::<u8>(), 0..2048)
    ) {
        let encrypted = encrypt(&key, &plaintext).unwrap();
        let decrypted = decrypt(&key, &encrypted).unwrap();
        prop_assert_eq!(decrypted, plaintext);

        let mut tampered = encrypted;
        let last = tampered.len() - 1;
        tampered[last] ^= 0x01;
        prop_assert!(decrypt(&key, &tampered).is_err());
    }
}

fn public_fields(agent_id: &str) -> CapsulePublic {
    CapsulePublic {
        agent_id: agent_id.to_string(),
        agent_version: Some("vector".to_string()),
        toolchain_digest: Some("sha256:test-vector".to_string()),
        env_fingerprint: Some("test-env".to_string()),
        evidence: vec![Evidence {
            name: "ci".to_string(),
            status: "pass".to_string(),
            duration_ms: 10,
            artifact_refs: vec!["artifact://crypto-vector".to_string()],
            summary: None,
            revision_id: None,
            command: None,
            exit_code: None,
            started_at_ms: None,
            ended_at_ms: None,
            environment_digest: None,
            runner_identity: None,
            log_digest: None,
            artifact_digest: None,
            expires_at_ms: None,
            trust_domain: None,
            signature: None,
        }],
    }
}

fn hex_array_32(input: &str) -> [u8; 32] {
    decode_hex(input).try_into().unwrap()
}

fn decode_hex(input: &str) -> Vec<u8> {
    assert!(
        input.len().is_multiple_of(2),
        "hex input must have even length"
    );
    (0..input.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&input[i..i + 2], 16).unwrap())
        .collect()
}
