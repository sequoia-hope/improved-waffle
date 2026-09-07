//! Persisted STEP payload encoding (roadmap §3.3): the feature stores the
//! source STEP text deflate-compressed + base64 inside the project JSON, so
//! a `.waffle` file is self-contained and the import replays on rebuild.

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine as _;
use std::io::{Read, Write};

/// The encoding tag written into the feature params. Bump only with a
/// decoder that accepts both.
pub const STEP_BLOB_ENCODING: &str = "deflate-base64";

/// Compress and encode STEP text for persistence.
pub fn encode_step_blob(step_text: &str) -> String {
    let mut enc = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
    // Writing into a Vec cannot fail.
    let _ = enc.write_all(step_text.as_bytes());
    let bytes = enc.finish().unwrap_or_default();
    B64.encode(bytes)
}

/// Largest payload a blob may inflate to (256 MiB). `.waffle` files are an
/// exchange format (shared documents, git-linked sources —
/// `specs/waffle_v4_document_model.md` §6), so a deflate bomb must be a loud
/// per-feature error, not an allocation abort (which on wasm32 is a hard
/// crash).
pub const MAX_INFLATED_BYTES: usize = 256 * 1024 * 1024;

/// Decode a persisted blob back to STEP text.
pub fn decode_step_blob(encoding: &str, data: &str) -> Result<String, String> {
    decode_step_blob_capped(encoding, data, MAX_INFLATED_BYTES)
}

/// [`decode_step_blob`] with an explicit inflation cap (bytes). Exceeding it
/// is `EmbedTooLarge`.
pub fn decode_step_blob_capped(encoding: &str, data: &str, cap: usize) -> Result<String, String> {
    if encoding != STEP_BLOB_ENCODING {
        return Err(format!("unknown STEP blob encoding '{encoding}'"));
    }
    let bytes = B64
        .decode(data)
        .map_err(|e| format!("STEP blob base64 decode failed: {e}"))?;
    // Read at most cap + 1 bytes: one byte past the cap proves the payload is
    // too large without ever inflating it fully.
    let mut raw = Vec::new();
    flate2::read::DeflateDecoder::new(bytes.as_slice())
        .take(cap as u64 + 1)
        .read_to_end(&mut raw)
        .map_err(|e| format!("STEP blob inflate failed: {e}"))?;
    if raw.len() > cap {
        return Err(format!(
            "EmbedTooLarge: payload inflates beyond the {cap}-byte cap"
        ));
    }
    String::from_utf8(raw).map_err(|e| format!("STEP blob is not UTF-8: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inflation_beyond_the_cap_is_a_loud_error_not_an_allocation() {
        // 1 MiB of zeros deflates to a few hundred bytes; a 16-byte cap
        // refuses it without inflating the whole thing.
        let text = "0".repeat(1024 * 1024);
        let blob = encode_step_blob(&text);
        let err = decode_step_blob_capped(STEP_BLOB_ENCODING, &blob, 16).unwrap_err();
        assert!(err.contains("EmbedTooLarge"), "{err}");
        // Exactly at the cap is fine.
        let small = "x".repeat(16);
        let blob = encode_step_blob(&small);
        assert_eq!(
            decode_step_blob_capped(STEP_BLOB_ENCODING, &blob, 16).unwrap(),
            small
        );
        // The default cap admits ordinary files.
        assert_eq!(decode_step_blob(STEP_BLOB_ENCODING, &blob).unwrap(), small);
    }

    #[test]
    fn blob_round_trip() {
        let text = include_str!("../tests/fixtures/cube.step");
        let blob = encode_step_blob(text);
        assert!(blob.len() < text.len(), "deflate should shrink STEP text");
        let back = decode_step_blob(STEP_BLOB_ENCODING, &blob).unwrap();
        assert_eq!(back, text);
    }

    #[test]
    fn unknown_encoding_is_loud() {
        assert!(decode_step_blob("gzip", "abc").is_err());
    }

    #[test]
    fn corrupt_data_is_loud() {
        assert!(decode_step_blob(STEP_BLOB_ENCODING, "!!!not-base64!!!").is_err());
    }
}
