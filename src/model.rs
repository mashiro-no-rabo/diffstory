use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Storyline {
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub description: Option<String>,
  pub groups: Vec<Group>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
  pub title: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub description: Option<String>,
  pub sections: Vec<Section>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Section {
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
