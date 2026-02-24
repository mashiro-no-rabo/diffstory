use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::io::{Read, Write};
use thiserror::Error;

use crate::model::Storyline;

#[derive(Debug, Error)]
pub enum CodecError {
  #[error("JSON error: {0}")]
  Json(#[from] serde_json::Error),
  #[error("IO error: {0}")]
  Io(#[from] std::io::Error),
  #[error("base64 decode error: {0}")]
  Base64(#[from] base64::DecodeError),
  #[error("diffstory marker not found in input")]
  MarkerNotFound,
}

const MARKER: &str = "<!--diffstory:";
const MARKER_END: &str = "-->";

/// Encode a storyline to base64-compressed string.
pub fn encode(storyline: &Storyline) -> Result<String, CodecError> {
  let json = serde_json::to_string(storyline)?;
  let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
  encoder.write_all(json.as_bytes())?;
  let compressed = encoder.finish()?;
  Ok(BASE64.encode(compressed))
}

/// Decode a base64-compressed string back to a storyline.
pub fn decode(encoded: &str) -> Result<Storyline, CodecError> {
  let compressed = BASE64.decode(encoded.trim())?;
  let mut decoder = GzDecoder::new(&compressed[..]);
  let mut json = String::new();
  decoder.read_to_string(&mut json)?;
  Ok(serde_json::from_str(&json)?)
}

/// Wrap encoded data in the PR-embeddable format.
pub fn wrap(encoded: &str) -> String {
  format!("<details><summary>diffstory</summary>\n\n{MARKER}{encoded}{MARKER_END}\n\n</details>")
}

/// Extract encoded data from PR description text.
pub fn extract_from_text(text: &str) -> Result<String, CodecError> {
  let start = text.find(MARKER).ok_or(CodecError::MarkerNotFound)?;
  let data_start = start + MARKER.len();
  let end = text[data_start..].find(MARKER_END).ok_or(CodecError::MarkerNotFound)?;
  Ok(text[data_start..data_start + end].to_string())
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::model::{Chapter, HunkRef};

  fn sample_storyline() -> Storyline {
    Storyline {
      description: Some("Test story".to_string()),
      chapters: vec![Chapter {
        title: "Chapter 1".to_string(),
        description: None,
        hunks: vec![HunkRef {
          file: "src/main.rs".to_string(),
          hunk_index: 0,
          note: Some("First change".to_string()),
        }],
      }],
      irrelevant: vec![],
    }
  }

  #[test]
  fn test_roundtrip() {
    let story = sample_storyline();
    let encoded = encode(&story).unwrap();
    let decoded = decode(&encoded).unwrap();
    assert_eq!(decoded.chapters.len(), 1);
    assert_eq!(decoded.chapters[0].title, "Chapter 1");
  }

  #[test]
  fn test_wrap_and_extract() {
    let story = sample_storyline();
    let encoded = encode(&story).unwrap();
    let wrapped = wrap(&encoded);
    let extracted = extract_from_text(&wrapped).unwrap();
    assert_eq!(encoded, extracted);
  }
}
