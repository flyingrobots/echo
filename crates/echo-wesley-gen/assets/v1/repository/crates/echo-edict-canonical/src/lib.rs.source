// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Exact `edict.canonical-cbor/v1` values, bytes, and artifact digests.
//!
//! This crate is a pure compatibility boundary for the Edict-owned provider
//! contract. It deliberately does not reuse Echo's WASM ABI codec: that codec
//! admits a different value model. The implementation accepts only the
//! definite-length Edict v1 subset, enforces the published nesting bound, and
//! validates decoded bytes by exact canonical re-encoding.

use std::collections::BTreeSet;
use std::fmt;
use std::str;

use sha2::{Digest, Sha256};

/// Coordinate of the canonical encoding profile implemented by this module.
pub const EDICT_CANONICAL_CBOR_V1: &str = "edict.canonical-cbor/v1";

/// Domain marker at the head of every Edict v1 artifact digest frame.
pub const EDICT_DIGEST_FRAME_V1: &str = "edict.digest/v1";

/// Maximum child nesting depth accepted by the Edict v1 encoder and decoder.
///
/// The root value is at depth zero. A scalar wrapped in exactly 128 containers
/// is accepted; one additional container is rejected.
pub const MAX_CANONICAL_NESTING_DEPTH_V1: usize = 128;

/// Maximum number of value nodes materialized by one canonical decode.
///
/// The root counts as one node. Array members, map keys, and map values each
/// count separately. This is an Echo host resource bound rather than part of
/// the Edict canonical byte identity; it prevents compact collection headers
/// from amplifying into unbounded host allocations before admission.
pub const MAX_CANONICAL_DECODE_NODES_V1: usize = 65_536;

/// Stable failure categories for Edict canonical values and byte streams.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CanonicalValueErrorKind {
    /// A value or collection length is outside the supported value model.
    UnsupportedValue,
    /// An integer is outside the CBOR major-zero or major-one `u64` range.
    InvalidInteger,
    /// The byte stream ended before one complete value was decoded.
    UnexpectedEof,
    /// Bytes remained after one complete canonical value.
    TrailingData,
    /// The byte stream used an unsupported CBOR major or additional-info form.
    UnsupportedCbor,
    /// The decoded value did not re-encode to the exact supplied bytes.
    NonCanonical,
    /// The value exceeded the published 128-level nesting bound.
    NestingLimitExceeded,
    /// A map contained two keys with identical canonical encodings.
    DuplicateMapKey,
    /// A CBOR text string was not valid UTF-8.
    InvalidUtf8,
}

impl CanonicalValueErrorKind {
    const fn label(self) -> &'static str {
        match self {
            Self::UnsupportedValue => "unsupported-value",
            Self::InvalidInteger => "invalid-integer",
            Self::UnexpectedEof => "unexpected-eof",
            Self::TrailingData => "trailing-data",
            Self::UnsupportedCbor => "unsupported-cbor",
            Self::NonCanonical => "noncanonical",
            Self::NestingLimitExceeded => "nesting-limit-exceeded",
            Self::DuplicateMapKey => "duplicate-map-key",
            Self::InvalidUtf8 => "invalid-utf8",
        }
    }
}

/// Structured canonical-value failure with a stable kind and diagnostic detail.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalValueError {
    kind: CanonicalValueErrorKind,
    detail: String,
}

impl CanonicalValueError {
    /// Returns the stable machine-readable failure category.
    #[must_use]
    pub const fn kind(&self) -> CanonicalValueErrorKind {
        self.kind
    }

    /// Returns deterministic diagnostic detail for the failed boundary.
    #[must_use]
    pub fn detail(&self) -> &str {
        &self.detail
    }

    fn new(kind: CanonicalValueErrorKind, detail: impl Into<String>) -> Self {
        Self {
            kind,
            detail: detail.into(),
        }
    }
}

impl fmt::Display for CanonicalValueError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.kind.label(), self.detail)
    }
}

impl std::error::Error for CanonicalValueError {}

/// Value tree admitted by `edict.canonical-cbor/v1`.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum CanonicalValueV1 {
    /// CBOR null.
    Null,
    /// CBOR false or true.
    Bool(bool),
    /// CBOR major-zero or major-one integer stored in a convenient host type.
    Integer(i128),
    /// Definite-length byte string.
    Bytes(Vec<u8>),
    /// Definite-length UTF-8 text string, without Unicode normalization.
    Text(String),
    /// Definite-length ordered array.
    Array(Vec<CanonicalValueV1>),
    /// Definite-length map sorted during encoding by canonical key bytes.
    Map(Vec<(CanonicalValueV1, CanonicalValueV1)>),
}

/// Encodes one value using the exact `edict.canonical-cbor/v1` byte contract.
///
/// # Errors
///
/// Returns a stable failure when an integer or collection cannot be represented,
/// a map repeats a canonical key, or the value exceeds the nesting bound.
pub fn encode_canonical_cbor_v1(value: &CanonicalValueV1) -> Result<Vec<u8>, CanonicalValueError> {
    let mut output = Vec::new();
    encode_value(value, &mut output, 0)?;
    Ok(output)
}

/// Decodes and authenticates one `edict.canonical-cbor/v1` value.
///
/// Decoding is followed by exact canonical re-encoding. This rejects
/// non-minimal integers and lengths, unsorted maps, and any other supported
/// value whose supplied bytes differ from the unique Edict encoding.
/// Successful decoding authenticates the encoding only: callers must still
/// validate the value against its owning CDDL root, and Echo must separately
/// admit any runtime artifact or authority.
///
/// # Errors
///
/// Returns a stable failure for malformed, unsupported, trailing,
/// noncanonical, duplicate-key, invalid-UTF-8, or over-nested input.
pub fn decode_canonical_cbor_v1(bytes: &[u8]) -> Result<CanonicalValueV1, CanonicalValueError> {
    let mut decoder = Decoder::new(bytes);
    let value = decoder.value(0)?;
    if decoder.remaining() != 0 {
        return Err(CanonicalValueError::new(
            CanonicalValueErrorKind::TrailingData,
            "canonical CBOR stream contains bytes after the first value",
        ));
    }
    if encode_canonical_cbor_v1(&value)? != bytes {
        return Err(CanonicalValueError::new(
            CanonicalValueErrorKind::NonCanonical,
            "decoded value does not re-encode to the supplied bytes",
        ));
    }
    Ok(value)
}

/// Computes an Edict v1 domain-framed SHA-256 digest for a canonical value.
///
/// The returned review rendering is `sha256:<64 lowercase hex>`. The preimage
/// is canonical CBOR for `["edict.digest/v1", domain, value]`. Each tuple member
/// is encoded at its own root so digest framing does not consume the artifact's
/// 128-level nesting budget.
/// A digest binds bytes and a caller-selected domain; it does not prove the
/// owning CDDL root or grant Echo runtime authority.
///
/// # Errors
///
/// Returns a stable failure for an empty domain or an unencodable value.
pub fn digest_canonical_value_v1(
    domain: &str,
    value: &CanonicalValueV1,
) -> Result<String, CanonicalValueError> {
    if domain.is_empty() {
        return Err(CanonicalValueError::new(
            CanonicalValueErrorKind::UnsupportedValue,
            "canonical artifact digest domain is empty",
        ));
    }

    let mut preimage = vec![0x83];
    preimage.extend(encode_canonical_cbor_v1(&CanonicalValueV1::Text(
        EDICT_DIGEST_FRAME_V1.to_owned(),
    ))?);
    preimage.extend(encode_canonical_cbor_v1(&CanonicalValueV1::Text(
        domain.to_owned(),
    ))?);
    preimage.extend(encode_canonical_cbor_v1(value)?);

    Ok(format!("sha256:{}", hex::encode(Sha256::digest(preimage))))
}

fn encode_value(
    value: &CanonicalValueV1,
    output: &mut Vec<u8>,
    depth: usize,
) -> Result<(), CanonicalValueError> {
    check_depth(depth)?;
    match value {
        CanonicalValueV1::Null => output.push(0xf6),
        CanonicalValueV1::Bool(false) => output.push(0xf4),
        CanonicalValueV1::Bool(true) => output.push(0xf5),
        CanonicalValueV1::Integer(value) => encode_integer(*value, output)?,
        CanonicalValueV1::Bytes(bytes) => {
            encode_type_value(2, usize_to_u64(bytes.len())?, output);
            output.extend_from_slice(bytes);
        }
        CanonicalValueV1::Text(text) => {
            encode_type_value(3, usize_to_u64(text.len())?, output);
            output.extend_from_slice(text.as_bytes());
        }
        CanonicalValueV1::Array(values) => {
            check_container_depth(depth)?;
            encode_type_value(4, usize_to_u64(values.len())?, output);
            for value in values {
                encode_value(value, output, depth + 1)?;
            }
        }
        CanonicalValueV1::Map(entries) => {
            check_container_depth(depth)?;
            let mut encoded_entries = Vec::with_capacity(entries.len());
            let mut encoded_keys = BTreeSet::new();
            for (key, value) in entries {
                let mut key_bytes = Vec::new();
                encode_value(key, &mut key_bytes, depth + 1)?;
                if !encoded_keys.insert(key_bytes.clone()) {
                    return Err(CanonicalValueError::new(
                        CanonicalValueErrorKind::DuplicateMapKey,
                        "canonical CBOR map contains duplicate keys",
                    ));
                }
                encoded_entries.push((key_bytes, value));
            }
            encoded_entries.sort_by(|(left, _), (right, _)| left.cmp(right));
            encode_type_value(5, usize_to_u64(encoded_entries.len())?, output);
            for (key_bytes, value) in encoded_entries {
                output.extend_from_slice(&key_bytes);
                encode_value(value, output, depth + 1)?;
            }
        }
    }
    Ok(())
}

fn check_depth(depth: usize) -> Result<(), CanonicalValueError> {
    if depth > MAX_CANONICAL_NESTING_DEPTH_V1 {
        return Err(CanonicalValueError::new(
            CanonicalValueErrorKind::NestingLimitExceeded,
            format!(
                "canonical value nesting exceeds maximum depth {MAX_CANONICAL_NESTING_DEPTH_V1}"
            ),
        ));
    }
    Ok(())
}

fn check_container_depth(depth: usize) -> Result<(), CanonicalValueError> {
    if depth >= MAX_CANONICAL_NESTING_DEPTH_V1 {
        return Err(CanonicalValueError::new(
            CanonicalValueErrorKind::NestingLimitExceeded,
            format!(
                "canonical container nesting exceeds maximum depth {MAX_CANONICAL_NESTING_DEPTH_V1}"
            ),
        ));
    }
    Ok(())
}

fn encode_integer(value: i128, output: &mut Vec<u8>) -> Result<(), CanonicalValueError> {
    if value >= 0 {
        let value = u64::try_from(value).map_err(|_| {
            CanonicalValueError::new(
                CanonicalValueErrorKind::InvalidInteger,
                "positive integer exceeds the canonical CBOR uint range",
            )
        })?;
        encode_type_value(0, value, output);
        return Ok(());
    }

    let magnitude = (-1i128).checked_sub(value).ok_or_else(|| {
        CanonicalValueError::new(
            CanonicalValueErrorKind::InvalidInteger,
            "negative integer cannot be converted to the canonical CBOR range",
        )
    })?;
    let magnitude = u64::try_from(magnitude).map_err(|_| {
        CanonicalValueError::new(
            CanonicalValueErrorKind::InvalidInteger,
            "negative integer exceeds the canonical CBOR negative range",
        )
    })?;
    encode_type_value(1, magnitude, output);
    Ok(())
}

fn encode_type_value(major: u8, value: u64, output: &mut Vec<u8>) {
    let prefix = major << 5;
    let bytes = value.to_be_bytes();
    match value {
        0..=23 => output.push(prefix | bytes[7]),
        24..=0xff => {
            output.push(prefix | 0x18);
            output.push(bytes[7]);
        }
        0x100..=0xffff => {
            output.push(prefix | 0x19);
            output.extend_from_slice(&bytes[6..]);
        }
        0x1_0000..=0xffff_ffff => {
            output.push(prefix | 0x1a);
            output.extend_from_slice(&bytes[4..]);
        }
        _ => {
            output.push(prefix | 0x1b);
            output.extend_from_slice(&bytes);
        }
    }
}

fn usize_to_u64(value: usize) -> Result<u64, CanonicalValueError> {
    u64::try_from(value).map_err(|_| {
        CanonicalValueError::new(
            CanonicalValueErrorKind::UnsupportedValue,
            "canonical collection length does not fit the CBOR uint range",
        )
    })
}

fn checked_collection_length<HostLength>(
    declared: u64,
    remaining: u64,
) -> Result<HostLength, CanonicalValueError>
where
    HostLength: TryFrom<u64>,
{
    if declared > remaining {
        return Err(CanonicalValueError::new(
            CanonicalValueErrorKind::UnexpectedEof,
            "canonical CBOR declared length exceeds the remaining bytes",
        ));
    }

    HostLength::try_from(declared).map_err(|_| {
        CanonicalValueError::new(
            CanonicalValueErrorKind::UnsupportedCbor,
            "canonical CBOR collection length does not fit usize",
        )
    })
}

struct Decoder<'a> {
    bytes: &'a [u8],
    position: usize,
    remaining_nodes: usize,
    remaining_reserved_nodes: usize,
}

impl<'a> Decoder<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self::with_node_budget(bytes, MAX_CANONICAL_DECODE_NODES_V1)
    }

    const fn with_node_budget(bytes: &'a [u8], node_budget: usize) -> Self {
        Self {
            bytes,
            position: 0,
            remaining_nodes: node_budget,
            remaining_reserved_nodes: node_budget,
        }
    }

    const fn remaining(&self) -> usize {
        self.bytes.len() - self.position
    }

    fn value(&mut self, depth: usize) -> Result<CanonicalValueV1, CanonicalValueError> {
        self.charge_nodes(1)?;
        check_depth(depth)?;
        let initial = self.byte()?;
        let major = initial >> 5;
        let additional = initial & 0x1f;
        match major {
            0 => Ok(CanonicalValueV1::Integer(i128::from(
                self.argument(additional)?,
            ))),
            1 => Ok(CanonicalValueV1::Integer(
                -1 - i128::from(self.argument(additional)?),
            )),
            2 => {
                let length = self.length(additional)?;
                Ok(CanonicalValueV1::Bytes(self.take(length)?.to_vec()))
            }
            3 => {
                let length = self.length(additional)?;
                let bytes = self.take(length)?;
                let text = str::from_utf8(bytes).map_err(|_| {
                    CanonicalValueError::new(
                        CanonicalValueErrorKind::InvalidUtf8,
                        "canonical CBOR text string is not valid UTF-8",
                    )
                })?;
                Ok(CanonicalValueV1::Text(text.to_owned()))
            }
            4 => {
                check_container_depth(depth)?;
                let length = self.length(additional)?;
                self.ensure_nodes_available(length)?;
                self.reserve_nodes(length)?;
                let mut values = Vec::with_capacity(length);
                for _ in 0..length {
                    values.push(self.value(depth + 1)?);
                }
                Ok(CanonicalValueV1::Array(values))
            }
            5 => {
                check_container_depth(depth)?;
                let length = self.length(additional)?;
                let child_nodes = length.checked_mul(2).ok_or_else(node_budget_error)?;
                self.ensure_nodes_available(child_nodes)?;
                self.reserve_nodes(child_nodes)?;
                let mut entries = Vec::with_capacity(length);
                let mut encoded_keys = BTreeSet::new();
                for _ in 0..length {
                    let key = self.value(depth + 1)?;
                    let mut key_bytes = Vec::new();
                    encode_value(&key, &mut key_bytes, depth + 1)?;
                    if !encoded_keys.insert(key_bytes) {
                        return Err(CanonicalValueError::new(
                            CanonicalValueErrorKind::DuplicateMapKey,
                            "canonical CBOR map contains duplicate keys",
                        ));
                    }
                    let value = self.value(depth + 1)?;
                    entries.push((key, value));
                }
                Ok(CanonicalValueV1::Map(entries))
            }
            7 => match additional {
                20 => Ok(CanonicalValueV1::Bool(false)),
                21 => Ok(CanonicalValueV1::Bool(true)),
                22 => Ok(CanonicalValueV1::Null),
                _ => Err(CanonicalValueError::new(
                    CanonicalValueErrorKind::UnsupportedCbor,
                    "canonical CBOR simple value is unsupported",
                )),
            },
            _ => Err(CanonicalValueError::new(
                CanonicalValueErrorKind::UnsupportedCbor,
                "canonical CBOR major type is unsupported",
            )),
        }
    }

    fn argument(&mut self, additional: u8) -> Result<u64, CanonicalValueError> {
        match additional {
            0..=23 => Ok(u64::from(additional)),
            24 => Ok(u64::from(self.byte()?)),
            25 => Ok(u64::from(u16::from_be_bytes(self.take_array::<2>()?))),
            26 => Ok(u64::from(u32::from_be_bytes(self.take_array::<4>()?))),
            27 => Ok(u64::from_be_bytes(self.take_array::<8>()?)),
            _ => Err(CanonicalValueError::new(
                CanonicalValueErrorKind::UnsupportedCbor,
                "indefinite or reserved canonical CBOR length is unsupported",
            )),
        }
    }

    fn length(&mut self, additional: u8) -> Result<usize, CanonicalValueError> {
        let declared = self.argument(additional)?;
        let remaining = u64::try_from(self.remaining()).map_err(|_| {
            CanonicalValueError::new(
                CanonicalValueErrorKind::UnsupportedCbor,
                "remaining canonical CBOR input does not fit the CBOR uint range",
            )
        })?;

        checked_collection_length::<usize>(declared, remaining)
    }

    fn ensure_nodes_available(&self, required: usize) -> Result<(), CanonicalValueError> {
        if required > self.remaining_nodes {
            return Err(node_budget_error());
        }
        Ok(())
    }

    fn charge_nodes(&mut self, count: usize) -> Result<(), CanonicalValueError> {
        self.remaining_nodes = self
            .remaining_nodes
            .checked_sub(count)
            .ok_or_else(node_budget_error)?;
        Ok(())
    }

    fn reserve_nodes(&mut self, count: usize) -> Result<(), CanonicalValueError> {
        self.remaining_reserved_nodes = self
            .remaining_reserved_nodes
            .checked_sub(count)
            .ok_or_else(node_budget_error)?;
        Ok(())
    }

    fn byte(&mut self) -> Result<u8, CanonicalValueError> {
        let Some(value) = self.bytes.get(self.position).copied() else {
            return Err(CanonicalValueError::new(
                CanonicalValueErrorKind::UnexpectedEof,
                "canonical CBOR expected another byte",
            ));
        };
        self.position += 1;
        Ok(value)
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], CanonicalValueError> {
        let end = self.position.checked_add(length).ok_or_else(|| {
            CanonicalValueError::new(
                CanonicalValueErrorKind::UnexpectedEof,
                "canonical CBOR length overflowed the input position",
            )
        })?;
        let Some(bytes) = self.bytes.get(self.position..end) else {
            return Err(CanonicalValueError::new(
                CanonicalValueErrorKind::UnexpectedEof,
                "canonical CBOR value extends past the input",
            ));
        };
        self.position = end;
        Ok(bytes)
    }

    fn take_array<const LENGTH: usize>(&mut self) -> Result<[u8; LENGTH], CanonicalValueError> {
        let mut output = [0u8; LENGTH];
        output.copy_from_slice(self.take(LENGTH)?);
        Ok(output)
    }
}

fn node_budget_error() -> CanonicalValueError {
    CanonicalValueError::new(
        CanonicalValueErrorKind::UnsupportedValue,
        "canonical CBOR decoded node budget exceeded",
    )
}

#[cfg(test)]
mod tests {
    use super::{checked_collection_length, CanonicalValueError, CanonicalValueErrorKind, Decoder};

    #[test]
    fn declared_length_bounds_precede_host_width_conversion() {
        assert_eq!(
            checked_collection_length::<u32>(u64::MAX, u64::from(u32::MAX)),
            Err(CanonicalValueError::new(
                CanonicalValueErrorKind::UnexpectedEof,
                "canonical CBOR declared length exceeds the remaining bytes"
            ))
        );
    }

    #[test]
    fn nested_container_reservations_are_cumulatively_bounded() {
        let bytes = [0x87, 0x86, 0x85, 0x84, 0x83, 0x82, 0x81, 0xf6];
        let mut decoder = Decoder::with_node_budget(&bytes, 8);

        assert_eq!(
            decoder.value(0),
            Err(CanonicalValueError::new(
                CanonicalValueErrorKind::UnsupportedValue,
                "canonical CBOR decoded node budget exceeded"
            ))
        );
        assert_eq!(
            decoder.position, 2,
            "the second container must be refused before its capacity is reserved"
        );
    }
}
