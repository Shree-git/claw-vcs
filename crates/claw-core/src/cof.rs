use crate::error::CoreError;
use crate::object::TypeTag;

const MAGIC: &[u8; 4] = b"CLW1";
/// Current Claw Object Format version written by this crate.
pub const COF_VERSION: u8 = 0x01;
/// Oldest Claw Object Format version this crate may read or migrate.
pub const MIN_READABLE_COF_VERSION: u8 = 0x01;
const KNOWN_FLAG_BITS: u8 = 0x03;

/// Read/write compatibility class for a COF version byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CofVersionSupport {
    /// The version is the crate's native read/write format.
    Native,
    /// The version is older but within the migration window.
    ReadViaMigration,
    /// The version is newer than this crate understands.
    UnsupportedFuture,
    /// The version is older than the supported migration floor.
    UnsupportedPast,
}

/// Planned handling for an observed COF version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CofMigrationPlan {
    source_version: u8,
    target_version: u8,
    support: CofVersionSupport,
}

impl CofMigrationPlan {
    /// Version byte found in the encoded object.
    pub fn source_version(self) -> u8 {
        self.source_version
    }

    /// Version byte this crate writes by default.
    pub fn target_version(self) -> u8 {
        self.target_version
    }

    /// Compatibility class for the source version.
    pub fn support(self) -> CofVersionSupport {
        self.support
    }

    /// Return true when the object can be read by this crate.
    pub fn can_read(self) -> bool {
        matches!(
            self.support,
            CofVersionSupport::Native | CofVersionSupport::ReadViaMigration
        )
    }

    /// Return true when writing this source version is supported.
    pub fn can_write_source_version(self) -> bool {
        matches!(self.support, CofVersionSupport::Native)
    }

    /// Return true when decoding requires an explicit migration step.
    pub fn requires_migration(self) -> bool {
        matches!(self.support, CofVersionSupport::ReadViaMigration)
    }
}

/// Classify a COF version byte for migration and compatibility checks.
pub fn classify_cof_version(version: u8) -> CofVersionSupport {
    if version == COF_VERSION {
        CofVersionSupport::Native
    } else if version > COF_VERSION {
        CofVersionSupport::UnsupportedFuture
    } else if version >= MIN_READABLE_COF_VERSION {
        CofVersionSupport::ReadViaMigration
    } else {
        CofVersionSupport::UnsupportedPast
    }
}

/// Return the migration plan for an observed COF version.
///
/// v0.1 writes only the native COF version. The plan API is intentionally
/// present before v2 exists so future readers can add old-version migrators
/// without changing the public compatibility contract.
pub fn cof_migration_plan(version: u8) -> CofMigrationPlan {
    CofMigrationPlan {
        source_version: version,
        target_version: COF_VERSION,
        support: classify_cof_version(version),
    }
}

/// Return the COF version byte after validating the magic prefix.
pub fn cof_version(data: &[u8]) -> Result<u8, CoreError> {
    if data.len() < 5 {
        return Err(CoreError::Deserialization(
            "data too short for COF version".into(),
        ));
    }
    if &data[..4] != MAGIC {
        return Err(CoreError::InvalidMagic);
    }
    Ok(data[4])
}

/// Compression marker stored in the COF header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Compression {
    /// Payload bytes are stored without compression.
    None = 0x00,
    /// Payload bytes are compressed with zstd.
    Zstd = 0x01,
}

impl Compression {
    fn from_u8(v: u8) -> Option<Self> {
        match v {
            0x00 => Some(Self::None),
            0x01 => Some(Self::Zstd),
            _ => None,
        }
    }
}

/// Bit flags stored in a COF header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CofFlags(u8);

impl CofFlags {
    /// Build flag bits from the current compression and encryption booleans.
    pub fn new(compressed: bool, encrypted: bool) -> Self {
        let mut bits = 0u8;
        if compressed {
            bits |= 0x01;
        }
        if encrypted {
            bits |= 0x02;
        }
        Self(bits)
    }

    /// Return the raw encoded flag bits.
    pub fn bits(&self) -> u8 {
        self.0
    }

    /// Return whether the compressed flag is set.
    pub fn is_compressed(&self) -> bool {
        self.0 & 0x01 != 0
    }

    /// Return whether the encrypted flag is set.
    pub fn is_encrypted(&self) -> bool {
        self.0 & 0x02 != 0
    }
}

fn encode_uvarint(mut value: u64, buf: &mut Vec<u8>) {
    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        buf.push(byte);
        if value == 0 {
            break;
        }
    }
}

fn decode_uvarint(data: &[u8], pos: &mut usize) -> Result<u64, CoreError> {
    let mut result: u64 = 0;
    let mut shift = 0u32;
    loop {
        if *pos >= data.len() {
            return Err(CoreError::Deserialization(
                "unexpected end of uvarint".into(),
            ));
        }
        let byte = data[*pos];
        *pos += 1;
        result |= ((byte & 0x7F) as u64) << shift;
        if byte & 0x80 == 0 {
            break;
        }
        shift += 7;
        if shift >= 64 {
            return Err(CoreError::Deserialization("uvarint overflow".into()));
        }
    }
    Ok(result)
}

/// Encode a payload into COF v1 format.
/// Format: [4B magic][1B version][1B type_tag][1B flags][1B compression][uvarint uncompressed_len][payload][4B CRC32]
pub fn cof_encode(type_tag: TypeTag, payload: &[u8]) -> Result<Vec<u8>, CoreError> {
    let compression = if payload.len() > 64 {
        Compression::Zstd
    } else {
        Compression::None
    };

    let compressed = match compression {
        Compression::None => payload.to_vec(),
        Compression::Zstd => {
            zstd::encode_all(payload, 3).map_err(|e| CoreError::Compression(e.to_string()))?
        }
    };

    let flags = CofFlags::new(
        compression != Compression::None,
        false, // encryption flag set at higher layer
    );

    let mut buf = Vec::with_capacity(4 + 4 + compressed.len() + 10 + 4);

    // Header
    buf.extend_from_slice(MAGIC);
    buf.push(COF_VERSION);
    buf.push(type_tag as u8);
    buf.push(flags.bits());
    buf.push(compression as u8);

    // Uncompressed length
    encode_uvarint(payload.len() as u64, &mut buf);

    // Payload
    buf.extend_from_slice(&compressed);

    // CRC32 of uncompressed payload (little endian) per spec
    let crc = crc32fast::hash(payload);
    buf.extend_from_slice(&crc.to_le_bytes());

    Ok(buf)
}

/// Decode COF v1 format, returning (TypeTag, decompressed payload).
pub fn cof_decode(data: &[u8]) -> Result<(TypeTag, Vec<u8>), CoreError> {
    if data.len() < 12 {
        return Err(CoreError::Deserialization("data too short for COF".into()));
    }

    // Check magic
    if &data[..4] != MAGIC {
        return Err(CoreError::InvalidMagic);
    }

    // Version
    let version = data[4];
    if !matches!(classify_cof_version(version), CofVersionSupport::Native) {
        return Err(CoreError::UnsupportedVersion(version));
    }

    // Type tag
    let type_tag = TypeTag::from_u8(data[5]).ok_or(CoreError::UnknownTypeTag(data[5]))?;

    let flags = data[6];
    if flags & !KNOWN_FLAG_BITS != 0 {
        return Err(CoreError::Deserialization(format!(
            "unknown COF flags: 0x{flags:02x}"
        )));
    }

    // Compression
    let compression = Compression::from_u8(data[7])
        .ok_or_else(|| CoreError::Deserialization(format!("unknown compression: {}", data[7])))?;

    // Uncompressed length
    let mut pos = 8;
    let uncompressed_len = decode_uvarint(data, &mut pos)? as usize;

    // CRC32 check: last 4 bytes
    if data.len() < pos + 4 {
        return Err(CoreError::Deserialization(
            "data too short for CRC32".into(),
        ));
    }
    let crc_offset = data.len() - 4;
    let expected_crc = u32::from_le_bytes([
        data[crc_offset],
        data[crc_offset + 1],
        data[crc_offset + 2],
        data[crc_offset + 3],
    ]);

    // Compressed payload
    let compressed = &data[pos..crc_offset];

    // Decompress
    let payload = match compression {
        Compression::None => compressed.to_vec(),
        Compression::Zstd => {
            zstd::decode_all(compressed).map_err(|e| CoreError::Decompression(e.to_string()))?
        }
    };

    if payload.len() != uncompressed_len {
        return Err(CoreError::Deserialization(format!(
            "COF length mismatch: header says {uncompressed_len}, decoded {}",
            payload.len()
        )));
    };

    // CRC32 of uncompressed payload per spec
    let actual_crc = crc32fast::hash(&payload);
    if expected_crc != actual_crc {
        return Err(CoreError::Crc32Mismatch {
            expected: expected_crc,
            actual: actual_crc,
        });
    }

    Ok((type_tag, payload))
}

/// Decode COF bytes and return the compatibility plan used for decoding.
///
/// Native v1 objects decode directly. Older readable versions will use this
/// entry point when a migrator is added; until then no older version exists in
/// the supported range. Future and unsupported-past versions fail closed.
pub fn cof_decode_with_migration(
    data: &[u8],
) -> Result<(TypeTag, Vec<u8>, CofMigrationPlan), CoreError> {
    let version = cof_version(data)?;
    let plan = cof_migration_plan(version);
    match plan.support {
        CofVersionSupport::Native => {
            let (type_tag, payload) = cof_decode(data)?;
            Ok((type_tag, payload, plan))
        }
        CofVersionSupport::ReadViaMigration => Err(CoreError::UnsupportedVersion(version)),
        CofVersionSupport::UnsupportedFuture | CofVersionSupport::UnsupportedPast => {
            Err(CoreError::UnsupportedVersion(version))
        }
    }
}

/// Peek at the type tag from COF-encoded data without fully decoding.
///
/// This is useful when the raw COF bytes will be forwarded over the wire
/// (e.g., pack uploads) and only the type tag is needed for metadata.
pub fn cof_peek_type_tag(data: &[u8]) -> Result<TypeTag, CoreError> {
    if data.len() < 8 {
        return Err(CoreError::Deserialization(
            "data too short for COF header".into(),
        ));
    }
    if &data[..4] != MAGIC {
        return Err(CoreError::InvalidMagic);
    }
    let version = data[4];
    if !matches!(classify_cof_version(version), CofVersionSupport::Native) {
        return Err(CoreError::UnsupportedVersion(version));
    }
    TypeTag::from_u8(data[5]).ok_or(CoreError::UnknownTypeTag(data[5]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_no_compression() {
        let payload = b"short";
        let encoded = cof_encode(TypeTag::Blob, payload).unwrap();
        let (tag, decoded) = cof_decode(&encoded).unwrap();
        assert_eq!(tag, TypeTag::Blob);
        assert_eq!(decoded, payload);
    }

    #[test]
    fn roundtrip_with_compression() {
        let payload = vec![b'a'; 1000];
        let encoded = cof_encode(TypeTag::Tree, &payload).unwrap();
        let (tag, decoded) = cof_decode(&encoded).unwrap();
        assert_eq!(tag, TypeTag::Tree);
        assert_eq!(decoded, payload);
    }

    #[test]
    fn crc_corruption_detected() {
        let payload = b"test data";
        let mut encoded = cof_encode(TypeTag::Blob, payload).unwrap();
        // Corrupt one byte in the payload area
        let mid = encoded.len() / 2;
        encoded[mid] ^= 0xFF;
        assert!(cof_decode(&encoded).is_err());
    }

    #[test]
    fn invalid_magic_rejected() {
        let mut data = cof_encode(TypeTag::Blob, b"test").unwrap();
        data[0] = b'X';
        assert!(matches!(cof_decode(&data), Err(CoreError::InvalidMagic)));
    }

    #[test]
    fn future_version_rejected_but_classified_for_migration() {
        let mut data = cof_encode(TypeTag::Blob, b"test").unwrap();
        data[4] = COF_VERSION + 1;
        assert_eq!(
            classify_cof_version(COF_VERSION + 1),
            CofVersionSupport::UnsupportedFuture
        );
        assert!(matches!(
            cof_decode(&data),
            Err(CoreError::UnsupportedVersion(version)) if version == COF_VERSION + 1
        ));
    }

    #[test]
    fn migration_plan_describes_native_read_write() {
        let plan = cof_migration_plan(COF_VERSION);
        assert_eq!(plan.source_version(), COF_VERSION);
        assert_eq!(plan.target_version(), COF_VERSION);
        assert_eq!(plan.support(), CofVersionSupport::Native);
        assert!(plan.can_read());
        assert!(plan.can_write_source_version());
        assert!(!plan.requires_migration());
    }

    #[test]
    fn migration_plan_rejects_future_versions() {
        let plan = cof_migration_plan(COF_VERSION + 1);
        assert_eq!(plan.support(), CofVersionSupport::UnsupportedFuture);
        assert!(!plan.can_read());
        assert!(!plan.can_write_source_version());
    }

    #[test]
    fn decode_with_migration_returns_native_plan() {
        let encoded = cof_encode(TypeTag::Blob, b"migration plan").unwrap();
        let (tag, payload, plan) = cof_decode_with_migration(&encoded).unwrap();
        assert_eq!(tag, TypeTag::Blob);
        assert_eq!(payload, b"migration plan");
        assert_eq!(plan.support(), CofVersionSupport::Native);
    }

    #[test]
    fn unknown_flags_are_rejected() {
        let mut data = cof_encode(TypeTag::Blob, b"test").unwrap();
        data[6] = 0x80;
        assert!(matches!(
            cof_decode(&data),
            Err(CoreError::Deserialization(_))
        ));
    }

    #[test]
    fn unknown_compression_marker_is_rejected() {
        let mut data = cof_encode(TypeTag::Blob, b"test").unwrap();
        data[7] = 0xff;
        assert!(matches!(
            cof_decode(&data),
            Err(CoreError::Deserialization(_))
        ));
    }

    #[test]
    fn length_mismatch_or_payload_mutation_is_rejected() {
        let mut data = cof_encode(TypeTag::Blob, b"test").unwrap();
        data[8] = data[8].saturating_add(1);
        assert!(cof_decode(&data).is_err());
    }

    #[test]
    fn all_type_tags_roundtrip() {
        for tag_val in 0x01..=0x0Cu8 {
            let tag = TypeTag::from_u8(tag_val).unwrap();
            let payload = format!("payload for {}", tag.name());
            let encoded = cof_encode(tag, payload.as_bytes()).unwrap();
            let (decoded_tag, decoded_payload) = cof_decode(&encoded).unwrap();
            assert_eq!(decoded_tag, tag);
            assert_eq!(decoded_payload, payload.as_bytes());
        }
    }

    #[test]
    fn peek_type_tag_matches_decode() {
        for tag_val in 0x01..=0x0Cu8 {
            let tag = TypeTag::from_u8(tag_val).unwrap();
            let encoded = cof_encode(tag, b"hello world").unwrap();
            let peeked = cof_peek_type_tag(&encoded).unwrap();
            assert_eq!(peeked, tag);
        }
    }

    #[test]
    fn peek_type_tag_rejects_short_data() {
        assert!(cof_peek_type_tag(&[0; 4]).is_err());
    }

    #[test]
    fn peek_type_tag_rejects_bad_magic() {
        let mut data = cof_encode(TypeTag::Blob, b"test").unwrap();
        data[0] = b'X';
        assert!(matches!(
            cof_peek_type_tag(&data),
            Err(CoreError::InvalidMagic)
        ));
    }
}
