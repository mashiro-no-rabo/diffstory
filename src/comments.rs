use std::collections::HashMap;

use serde::Deserialize;

use crate::diff_parser::{DiffLine, ParsedDiff};

#[derive(Debug, Clone, Deserialize)]
pub struct ReviewComment {
    pub id: u64,
    pub path: String,
    pub line: Option<u32>,
    pub original_line: Option<u32>,
    #[serde(default)]
    pub side: Option<String>,
    pub body: String,
    pub user: CommentUser,
    pub created_at: String,
    pub in_reply_to_id: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IssueComment {
    pub id: u64,
    pub body: String,
    pub user: CommentUser,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CommentUser {
    pub login: String,
}

/// A review comment mapped to a specific hunk position.
#[derive(Debug, Clone)]
pub struct MappedComment {
    pub comment: ReviewComment,
    /// Offset within the hunk's lines where this comment applies.
    pub line_offset: usize,
    /// True if mapped via original_line (comment may be stale).
    pub is_outdated: bool,
}

/// A thread of review comments (root + replies).
#[derive(Debug, Clone)]
pub struct CommentThread {
    pub root: MappedComment,
    pub replies: Vec<ReviewComment>,
}

/// Key: (file_path, hunk_index) → list of threads on that hunk.
pub type CommentMap = HashMap<(String, usize), Vec<CommentThread>>;

/// A review comment that couldn't be mapped to any current hunk.
#[derive(Debug, Clone)]
pub struct OutdatedComment {
    pub comment: ReviewComment,
    pub file: String,
}

/// Parse `@@ -a,b +c,d @@` header into (old_start, old_count, new_start, new_count).
fn parse_hunk_header(header: &str) -> Option<(u32, u32, u32, u32)> {
    // Find the @@ ... @@ portion
    let header = header.strip_prefix("@@ ")?;
    let end = header.find(" @@")?;
    let range_str = &header[..end];

    let mut parts = range_str.split(' ');
    let old_part = parts.next()?.strip_prefix('-')?;
    let new_part = parts.next()?.strip_prefix('+')?;

    let (old_start, old_count) = parse_range(old_part)?;
    let (new_start, new_count) = parse_range(new_part)?;

    Some((old_start, old_count, new_start, new_count))
}

fn parse_range(s: &str) -> Option<(u32, u32)> {
    if let Some((start, count)) = s.split_once(',') {
        Some((start.parse().ok()?, count.parse().ok()?))
    } else {
        Some((s.parse().ok()?, 1))
    }
}

/// Map review comments to hunks in the parsed diff.
///
/// Returns (mapped comments by hunk, unmappable outdated comments).
pub fn map_comments_to_hunks(
    comments: Vec<ReviewComment>,
    diff: &ParsedDiff,
) -> (CommentMap, Vec<OutdatedComment>) {
    // Separate root comments from replies
    let mut roots: Vec<ReviewComment> = Vec::new();
    let mut replies: HashMap<u64, Vec<ReviewComment>> = HashMap::new();

    for c in comments {
        if let Some(reply_to) = c.in_reply_to_id {
            replies.entry(reply_to).or_default().push(c);
        } else {
            roots.push(c);
        }
    }

    // Sort replies by created_at
    for reply_list in replies.values_mut() {
        reply_list.sort_by(|a, b| a.created_at.cmp(&b.created_at));
    }

    let mut comment_map: CommentMap = HashMap::new();
    let mut outdated: Vec<OutdatedComment> = Vec::new();

    for root in roots {
        let root_replies = replies.remove(&root.id).unwrap_or_default();

        match try_map_comment(&root, diff) {
            Some((file_path, hunk_index, line_offset, is_outdated)) => {
                let key = (file_path, hunk_index);
                let thread = CommentThread {
                    root: MappedComment {
                        comment: root,
                        line_offset,
                        is_outdated,
                    },
                    replies: root_replies,
                };
                comment_map.entry(key).or_default().push(thread);
            }
            None => {
                let file = root.path.clone();
                // Also collect replies as outdated (they'll be visible in the thread context)
                outdated.push(OutdatedComment {
                    comment: root,
                    file,
                });
            }
        }
    }

    // Sort threads within each hunk by line_offset
    for threads in comment_map.values_mut() {
        threads.sort_by_key(|t| t.root.line_offset);
    }

    (comment_map, outdated)
}

/// Try to map a single comment to a (file_path, hunk_index, line_offset, is_outdated).
fn try_map_comment(
    comment: &ReviewComment,
    diff: &ParsedDiff,
) -> Option<(String, usize, usize, bool)> {
    let file_diff = diff
        .files
        .iter()
        .find(|f| f.display_path() == comment.path)?;

    let file_path = file_diff.display_path().to_string();

    // Strategy 1: Use `line` (current position) — not outdated
    if let Some(line_num) = comment.line {
        if let Some((hunk_idx, offset)) =
            find_line_in_hunks_new(&file_diff.hunks, line_num, &comment.side)
        {
            return Some((file_path, hunk_idx, offset, false));
        }
    }

    // Strategy 2: Use `original_line` — mark as outdated
    if let Some(orig_line) = comment.original_line {
        if let Some((hunk_idx, offset)) =
            find_line_in_hunks_original(&file_diff.hunks, orig_line, &comment.side)
        {
            return Some((file_path, hunk_idx, offset, true));
        }
    }

    None
}

/// Find which hunk contains a given new-side line number, return (hunk_index, line_offset).
fn find_line_in_hunks_new(
    hunks: &[crate::diff_parser::Hunk],
    target_line: u32,
    side: &Option<String>,
) -> Option<(usize, usize)> {
    let is_left = side.as_deref() == Some("LEFT");

    for (hunk_idx, hunk) in hunks.iter().enumerate() {
        let (old_start, _old_count, new_start, _new_count) = parse_hunk_header(&hunk.header)?;

        let mut old_line = old_start;
        let mut new_line = new_start;

        for (offset, diff_line) in hunk.lines.iter().enumerate() {
            match diff_line {
                DiffLine::Context(_) => {
                    if is_left && old_line == target_line {
                        return Some((hunk_idx, offset));
                    }
                    if !is_left && new_line == target_line {
                        return Some((hunk_idx, offset));
                    }
                    old_line += 1;
                    new_line += 1;
                }
                DiffLine::Addition(_) => {
                    if !is_left && new_line == target_line {
                        return Some((hunk_idx, offset));
                    }
                    new_line += 1;
                }
                DiffLine::Deletion(_) => {
                    if is_left && old_line == target_line {
                        return Some((hunk_idx, offset));
                    }
                    old_line += 1;
                }
                DiffLine::NoNewlineAtEof => {}
            }
        }
    }
    None
}

/// Find which hunk contains a given original-side line number (for outdated comments).
fn find_line_in_hunks_original(
    hunks: &[crate::diff_parser::Hunk],
    target_line: u32,
    side: &Option<String>,
) -> Option<(usize, usize)> {
    // For original_line, we look at old-side line numbers regardless of side hint
    let is_left = side.as_deref() != Some("RIGHT");

    for (hunk_idx, hunk) in hunks.iter().enumerate() {
        let (old_start, _old_count, new_start, _new_count) = parse_hunk_header(&hunk.header)?;

        let mut old_line = old_start;
        let mut new_line = new_start;

        for (offset, diff_line) in hunk.lines.iter().enumerate() {
            match diff_line {
                DiffLine::Context(_) => {
                    if is_left && old_line == target_line {
                        return Some((hunk_idx, offset));
                    }
                    if !is_left && new_line == target_line {
                        return Some((hunk_idx, offset));
                    }
                    old_line += 1;
                    new_line += 1;
                }
                DiffLine::Deletion(_) => {
                    if is_left && old_line == target_line {
                        return Some((hunk_idx, offset));
                    }
                    old_line += 1;
                }
                DiffLine::Addition(_) => {
                    if !is_left && new_line == target_line {
                        return Some((hunk_idx, offset));
                    }
                    new_line += 1;
                }
                DiffLine::NoNewlineAtEof => {}
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hunk_header() {
        assert_eq!(parse_hunk_header("@@ -1,3 +1,4 @@"), Some((1, 3, 1, 4)));
        assert_eq!(
            parse_hunk_header("@@ -10,3 +11,4 @@ fn main()"),
            Some((10, 3, 11, 4))
        );
        assert_eq!(parse_hunk_header("@@ -0,0 +1,3 @@"), Some((0, 0, 1, 3)));
        assert_eq!(parse_hunk_header("@@ -1 +1 @@"), Some((1, 1, 1, 1)));
    }

    #[test]
    fn test_map_comment_to_hunk() {
        let diff_text = "\
diff --git a/src/main.rs b/src/main.rs
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,3 +1,4 @@
 fn main() {
+    println!(\"hello\");
     println!(\"world\");
 }";
        let parsed = crate::diff_parser::parse_diff(diff_text).unwrap();

        let comment = ReviewComment {
            id: 1,
            path: "src/main.rs".to_string(),
            line: Some(2), // the added line
            original_line: None,
            side: Some("RIGHT".to_string()),
            body: "Nice addition!".to_string(),
            user: CommentUser {
                login: "reviewer".to_string(),
            },
            created_at: "2024-01-01T00:00:00Z".to_string(),
            in_reply_to_id: None,
        };

        let (map, outdated) = map_comments_to_hunks(vec![comment], &parsed);
        assert!(outdated.is_empty());
        assert_eq!(map.len(), 1);
        let threads = map.get(&("src/main.rs".to_string(), 0)).unwrap();
        assert_eq!(threads.len(), 1);
        assert_eq!(threads[0].root.line_offset, 1); // offset 1 = the addition line
        assert!(!threads[0].root.is_outdated);
    }

    #[test]
    fn test_threaded_comments() {
        let diff_text = "\
diff --git a/lib.rs b/lib.rs
--- a/lib.rs
+++ b/lib.rs
@@ -1,3 +1,4 @@
 use std::io;
+use std::fs;

 fn read() {}";
        let parsed = crate::diff_parser::parse_diff(diff_text).unwrap();

        let root = ReviewComment {
            id: 10,
            path: "lib.rs".to_string(),
            line: Some(2),
            original_line: None,
            side: Some("RIGHT".to_string()),
            body: "Why this import?".to_string(),
            user: CommentUser {
                login: "alice".to_string(),
            },
            created_at: "2024-01-01T00:00:00Z".to_string(),
            in_reply_to_id: None,
        };

        let reply = ReviewComment {
            id: 11,
            path: "lib.rs".to_string(),
            line: Some(2),
            original_line: None,
            side: Some("RIGHT".to_string()),
            body: "For file operations".to_string(),
            user: CommentUser {
                login: "bob".to_string(),
            },
            created_at: "2024-01-01T01:00:00Z".to_string(),
            in_reply_to_id: Some(10),
        };

        let (map, _) = map_comments_to_hunks(vec![root, reply], &parsed);
        let threads = map.get(&("lib.rs".to_string(), 0)).unwrap();
        assert_eq!(threads.len(), 1);
        assert_eq!(threads[0].replies.len(), 1);
        assert_eq!(threads[0].replies[0].body, "For file operations");
    }

    #[test]
    fn test_unmappable_comment_becomes_outdated() {
        let diff_text = "\
diff --git a/src/main.rs b/src/main.rs
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,3 +1,4 @@
 fn main() {
+    println!(\"hello\");
     println!(\"world\");
 }";
        let parsed = crate::diff_parser::parse_diff(diff_text).unwrap();

        let comment = ReviewComment {
            id: 1,
            path: "src/main.rs".to_string(),
            line: None,
            original_line: Some(100), // line 100 doesn't exist in any hunk
            side: None,
            body: "Old comment".to_string(),
            user: CommentUser {
                login: "reviewer".to_string(),
            },
            created_at: "2024-01-01T00:00:00Z".to_string(),
            in_reply_to_id: None,
        };

        let (map, outdated) = map_comments_to_hunks(vec![comment], &parsed);
        assert!(map.is_empty());
        assert_eq!(outdated.len(), 1);
        assert_eq!(outdated[0].file, "src/main.rs");
    }
}
