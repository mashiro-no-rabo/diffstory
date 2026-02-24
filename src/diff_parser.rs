use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
  #[error("unexpected diff format: {0}")]
  UnexpectedFormat(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedDiff {
  pub files: Vec<FileDiff>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileDiff {
  pub old_path: Option<String>,
  pub new_path: Option<String>,
  pub is_rename: bool,
  pub is_binary: bool,
  pub hunks: Vec<Hunk>,
}

impl FileDiff {
  /// Returns the most relevant path for display purposes.
  pub fn display_path(&self) -> &str {
    self
      .new_path
      .as_deref()
      .or(self.old_path.as_deref())
      .unwrap_or("<unknown>")
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hunk {
  pub header: String,
  pub lines: Vec<DiffLine>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffLine {
  Context(String),
  Addition(String),
  Deletion(String),
  NoNewlineAtEof,
}

pub fn parse_diff(input: &str) -> Result<ParsedDiff, ParseError> {
  let mut files = Vec::new();
  let lines: Vec<&str> = input.lines().collect();
  let mut i = 0;

  while i < lines.len() {
    if lines[i].starts_with("diff --git ") {
      let (file_diff, next_i) = parse_file_diff(&lines, i)?;
      files.push(file_diff);
      i = next_i;
    } else {
      i += 1;
    }
  }

  Ok(ParsedDiff { files })
}

fn parse_file_diff(lines: &[&str], start: usize) -> Result<(FileDiff, usize), ParseError> {
  let diff_line = lines[start];

  // Extract paths from "diff --git a/path b/path"
  let (a_path, b_path) = parse_diff_git_line(diff_line)?;

  let mut old_path = Some(a_path);
  let mut new_path = Some(b_path);
  let mut is_rename = false;
  let mut is_binary = false;
  let mut hunks = Vec::new();
  let mut i = start + 1;

  // Parse extended headers
  while i < lines.len() && !lines[i].starts_with("diff --git ") {
    let line = lines[i];
    if line.starts_with("rename from ") {
      is_rename = true;
      old_path = Some(line.strip_prefix("rename from ").unwrap().to_string());
    } else if line.starts_with("rename to ") {
      is_rename = true;
      new_path = Some(line.strip_prefix("rename to ").unwrap().to_string());
    } else if line.starts_with("Binary files") || line == "GIT binary patch" {
      is_binary = true;
    } else if line.starts_with("--- ") {
      let path = line.strip_prefix("--- ").unwrap();
      if path == "/dev/null" {
        old_path = None;
      } else {
        old_path = Some(strip_prefix_segment(path));
      }
    } else if line.starts_with("+++ ") {
      let path = line.strip_prefix("+++ ").unwrap();
      if path == "/dev/null" {
        new_path = None;
      } else {
        new_path = Some(strip_prefix_segment(path));
      }
    } else if line.starts_with("@@ ") {
      let (hunk, next_i) = parse_hunk(lines, i);
      hunks.push(hunk);
      i = next_i;
      continue;
    }
    // Skip other extended headers (index, mode, similarity, etc.)
    i += 1;
  }

  Ok((
    FileDiff {
      old_path,
      new_path,
      is_rename,
      is_binary,
      hunks,
    },
    i,
  ))
}

fn parse_diff_git_line(line: &str) -> Result<(String, String), ParseError> {
  // "diff --git a/path b/path"
  let rest = line
    .strip_prefix("diff --git ")
    .ok_or_else(|| ParseError::UnexpectedFormat(line.to_string()))?;

  // Handle paths with spaces: a/ prefix and b/ prefix
  // Find the " b/" separator - scan for " b/" where the b/ part matches
  if let Some(a_rest) = rest.strip_prefix("a/") {
    // Find " b/" separator
    if let Some(pos) = a_rest.find(" b/") {
      let a_path = a_rest[..pos].to_string();
      let b_path = a_rest[pos + 3..].to_string();
      return Ok((a_path, b_path));
    }
  }

  Err(ParseError::UnexpectedFormat(line.to_string()))
}

/// Strip "a/" or "b/" prefix from paths like "a/src/main.rs"
fn strip_prefix_segment(path: &str) -> String {
  if let Some(rest) = path.strip_prefix("a/").or_else(|| path.strip_prefix("b/")) {
    rest.to_string()
  } else {
    path.to_string()
  }
}

fn parse_hunk(lines: &[&str], start: usize) -> (Hunk, usize) {
  let header = lines[start].to_string();
  let mut diff_lines = Vec::new();
  let mut i = start + 1;

  while i < lines.len() {
    let line = lines[i];
    if line.starts_with("diff --git ") || line.starts_with("@@ ") {
      break;
    }
    match line.as_bytes().first() {
      Some(b'+') => diff_lines.push(DiffLine::Addition(line[1..].to_string())),
      Some(b'-') => diff_lines.push(DiffLine::Deletion(line[1..].to_string())),
      Some(b' ') => diff_lines.push(DiffLine::Context(line[1..].to_string())),
      Some(b'\\') => diff_lines.push(DiffLine::NoNewlineAtEof),
      _ => {
        // Empty context line (just a space that got trimmed, or truly empty)
        if line.is_empty() {
          diff_lines.push(DiffLine::Context(String::new()));
        } else {
          // Unknown line - stop parsing this hunk
          break;
        }
      }
    }
    i += 1;
  }

  (
    Hunk {
      header,
      lines: diff_lines,
    },
    i,
  )
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_simple_diff() {
    let diff = "\
diff --git a/src/main.rs b/src/main.rs
index abc1234..def5678 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,3 +1,4 @@
 fn main() {
+    println!(\"hello\");
     println!(\"world\");
 }";
    let parsed = parse_diff(diff).unwrap();
    assert_eq!(parsed.files.len(), 1);
    assert_eq!(parsed.files[0].display_path(), "src/main.rs");
    assert_eq!(parsed.files[0].hunks.len(), 1);
    assert_eq!(parsed.files[0].hunks[0].lines.len(), 4);
  }

  #[test]
  fn test_rename_detection() {
    let diff = "\
diff --git a/old.rs b/new.rs
similarity index 95%
rename from old.rs
rename to new.rs
index abc1234..def5678 100644
--- a/old.rs
+++ b/new.rs
@@ -1,3 +1,3 @@
-fn old() {}
+fn new() {}
 fn shared() {}";
    let parsed = parse_diff(diff).unwrap();
    assert_eq!(parsed.files.len(), 1);
    assert!(parsed.files[0].is_rename);
    assert_eq!(parsed.files[0].old_path.as_deref(), Some("old.rs"));
    assert_eq!(parsed.files[0].new_path.as_deref(), Some("new.rs"));
  }

  #[test]
  fn test_binary_file() {
    let diff = "\
diff --git a/image.png b/image.png
new file mode 100644
index 0000000..abc1234
Binary files /dev/null and b/image.png differ";
    let parsed = parse_diff(diff).unwrap();
    assert_eq!(parsed.files.len(), 1);
    assert!(parsed.files[0].is_binary);
  }

  #[test]
  fn test_multiple_hunks() {
    let diff = "\
diff --git a/lib.rs b/lib.rs
--- a/lib.rs
+++ b/lib.rs
@@ -1,3 +1,4 @@
 use std::io;
+use std::fs;

 fn read() {}
@@ -10,3 +11,4 @@
 fn write() {}
+fn delete() {}
 fn update() {}";
    let parsed = parse_diff(diff).unwrap();
    assert_eq!(parsed.files[0].hunks.len(), 2);
  }

  #[test]
  fn test_new_file() {
    let diff = "\
diff --git a/new.rs b/new.rs
new file mode 100644
index 0000000..abc1234
--- /dev/null
+++ b/new.rs
@@ -0,0 +1,3 @@
+fn new_function() {
+    todo!()
+}";
    let parsed = parse_diff(diff).unwrap();
    assert_eq!(parsed.files.len(), 1);
    assert!(parsed.files[0].old_path.is_none());
    assert_eq!(parsed.files[0].new_path.as_deref(), Some("new.rs"));
  }
}
