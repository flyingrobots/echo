// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Shared output formatting for text and JSON modes.

use crate::cli::OutputFormat;

/// Emits output in the selected format.
///
/// - `Text` mode prints `text` as-is (caller includes newlines).
/// - `Json` mode pretty-prints `json` with a trailing newline.
pub fn emit(format: &OutputFormat, text: &str, json: &serde_json::Value) {
    match format {
        OutputFormat::Text => print!("{text}"),
        OutputFormat::Json => match serde_json::to_string_pretty(json) {
            Ok(s) => println!("{s}"),
            Err(e) => eprintln!("error: failed to serialize JSON output: {e}"),
        },
    }
}

/// Formats a 32-byte hash as lowercase hex.
pub fn hex_hash(hash: &[u8; 32]) -> String {
    hex::encode(hash)
}

/// Formats a hash as a short hex prefix (first 8 chars).
pub fn short_hex(hash: &[u8; 32]) -> String {
    hex::encode(&hash[..4])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_hash_produces_64_chars() {
        let hash = [0xAB; 32];
        let hex = hex_hash(&hash);
        assert_eq!(hex.len(), 64);
        assert_eq!(&hex[..4], "abab");
    }

    #[test]
    fn short_hex_produces_8_chars() {
        let hash = [0xCD; 32];
        let short = short_hex(&hash);
        assert_eq!(short.len(), 8);
        assert_eq!(short, "cdcdcdcd");
    }
}
