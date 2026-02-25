use std::collections::{HashMap, HashSet};

use crate::comments::{CommentMap, CommentThread, GqlReviewThread, IssueComment, OutdatedComment};
use crate::diff_parser::{FileDiff, Hunk, ParsedDiff};
use crate::model::{HunkRef, Storyline};

#[derive(Debug)]
pub struct ResolvedStory {
  pub description: Option<String>,
  pub chapters: Vec<ResolvedChapter>,
  pub misc: Vec<ResolvedChapter>,
  pub uncategorized: Vec<UncategorizedHunk>,
  pub warnings: Vec<String>,
  pub issue_comments: Vec<IssueComment>,
  pub outdated_comments: Vec<OutdatedComment>,
  pub resolved_threads: Vec<GqlReviewThread>,
  pub bot_review_threads: Vec<GqlReviewThread>,
  pub bot_issue_comments: Vec<IssueComment>,
}

#[derive(Debug)]
pub struct ResolvedChapter {
  pub title: String,
  pub description: Option<String>,
  pub hunks: Vec<ResolvedHunk>,
}

#[derive(Debug)]
pub struct ResolvedHunk {
  pub file_path: String,
  pub file_diff: FileDiff,
  pub hunk: Hunk,
  pub hunk_index: usize,
  pub note: Option<String>,
  pub comments: Vec<CommentThread>,
}

#[derive(Debug)]
pub struct UncategorizedHunk {
  pub file_path: String,
  pub file_diff: FileDiff,
  pub hunk: Hunk,
  pub hunk_index: usize,
  pub comments: Vec<CommentThread>,
}

/// Key for tracking which hunks have been referenced.
type HunkKey = (String, usize);

pub fn resolve(storyline: &Storyline, diff: &ParsedDiff) -> ResolvedStory {
  resolve_with_comments(storyline, diff, None, Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new())
}

pub fn resolve_with_comments(
  storyline: &Storyline,
  diff: &ParsedDiff,
  comments: Option<CommentMap>,
  issue_comments: Vec<IssueComment>,
  outdated_comments: Vec<OutdatedComment>,
  resolved_threads: Vec<GqlReviewThread>,
  bot_review_threads: Vec<GqlReviewThread>,
  bot_issue_comments: Vec<IssueComment>,
) -> ResolvedStory {
  let mut warnings = Vec::new();
  let mut referenced: HashSet<HunkKey> = HashSet::new();
  let mut comment_map = comments.unwrap_or_default();

  // Build lookup: file path -> &FileDiff
  let file_map: HashMap<&str, &FileDiff> = diff.files.iter().map(|f| (f.display_path(), f)).collect();

  // Resolve chapters
  let chapters = resolve_chapters(&storyline.chapters, &file_map, &mut referenced, &mut warnings, &mut comment_map);

  // Resolve misc chapters
  let misc = resolve_chapters(&storyline.misc, &file_map, &mut referenced, &mut warnings, &mut comment_map);

  // Find uncategorized hunks
  let mut uncategorized = Vec::new();
  for file_diff in &diff.files {
    let path = file_diff.display_path();
    for (idx, hunk) in file_diff.hunks.iter().enumerate() {
      let key = (path.to_string(), idx);
      if !referenced.contains(&key) {
        let hunk_comments = comment_map.remove(&key).unwrap_or_default();
        uncategorized.push(UncategorizedHunk {
          file_path: path.to_string(),
          file_diff: file_diff.clone(),
          hunk: hunk.clone(),
          hunk_index: idx,
          comments: hunk_comments,
        });
      }
    }
  }

  ResolvedStory {
    description: storyline.description.clone(),
    chapters,
    misc,
    uncategorized,
    warnings,
    issue_comments,
    outdated_comments,
    resolved_threads,
    bot_review_threads,
    bot_issue_comments,
  }
}

fn resolve_chapters(
  chapters: &[crate::model::Chapter],
  file_map: &HashMap<&str, &FileDiff>,
  referenced: &mut HashSet<HunkKey>,
  warnings: &mut Vec<String>,
  comment_map: &mut CommentMap,
) -> Vec<ResolvedChapter> {
  chapters
    .iter()
    .map(|ch| {
      let hunks = ch
        .hunks
        .iter()
        .filter_map(|href| resolve_hunk_ref(href, file_map, referenced, warnings, comment_map))
        .collect();
      ResolvedChapter {
        title: ch.title.clone(),
        description: ch.description.clone(),
        hunks,
      }
    })
    .collect()
}

fn resolve_hunk_ref(
  href: &HunkRef,
  file_map: &HashMap<&str, &FileDiff>,
  referenced: &mut HashSet<HunkKey>,
  warnings: &mut Vec<String>,
  comment_map: &mut CommentMap,
) -> Option<ResolvedHunk> {
  let key = (href.file.clone(), href.hunk_index);

  if referenced.contains(&key) {
    warnings.push(format!("duplicate reference: {}:{}", href.file, href.hunk_index));
    return None;
  }

  match file_map.get(href.file.as_str()) {
    None => {
      warnings.push(format!("file not found in diff: {}", href.file));
      None
    }
    Some(file_diff) => {
      if href.hunk_index >= file_diff.hunks.len() {
        warnings.push(format!(
          "hunk index {} out of bounds for {} (has {} hunks)",
          href.hunk_index,
          href.file,
          file_diff.hunks.len()
        ));
        None
      } else {
        referenced.insert(key.clone());
        let hunk_comments = comment_map.remove(&key).unwrap_or_default();
        Some(ResolvedHunk {
          file_path: href.file.clone(),
          file_diff: (*file_diff).clone(),
          hunk: file_diff.hunks[href.hunk_index].clone(),
          hunk_index: href.hunk_index,
          note: href.note.clone(),
          comments: hunk_comments,
        })
      }
    }
  }
}

/// Validate a storyline against a diff and return coverage info.
pub struct ValidationResult {
  pub total_hunks: usize,
  pub covered_hunks: usize,
  pub uncategorized_hunks: usize,
  pub warnings: Vec<String>,
}

impl ValidationResult {
  pub fn coverage_pct(&self) -> f64 {
    if self.total_hunks == 0 {
      100.0
    } else {
      (self.covered_hunks as f64 / self.total_hunks as f64) * 100.0
    }
  }
}

pub fn validate(storyline: &Storyline, diff: &ParsedDiff) -> ValidationResult {
  let resolved = resolve(storyline, diff);
  let total_hunks: usize = diff.files.iter().map(|f| f.hunks.len()).sum();
  let uncategorized = resolved.uncategorized.len();
  let covered = total_hunks - uncategorized;

  ValidationResult {
    total_hunks,
    covered_hunks: covered,
    uncategorized_hunks: uncategorized,
    warnings: resolved.warnings,
  }
}
