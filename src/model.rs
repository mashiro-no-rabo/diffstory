use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Storyline {
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub description: Option<String>,
  pub chapters: Vec<Chapter>,
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub misc: Vec<Chapter>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chapter {
  pub title: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub description: Option<String>,
  pub hunks: Vec<HunkRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HunkRef {
  pub file: String,
  pub hunk_index: usize,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub note: Option<String>,
}
