use std::process::Command;

use thiserror::Error;

use crate::codec;
use crate::comments::{IssueComment, ReviewComment};

#[derive(Debug, Error)]
pub enum GithubError {
    #[error("gh CLI not found — install from https://cli.github.com/")]
    GhNotFound,
    #[error("gh command failed: {0}")]
    GhFailed(String),
    #[error("failed to extract storyline from PR body")]
    NoStoryline,
    #[error("not a valid GitHub PR URL: {0}")]
    InvalidPrUrl(String),
    #[error("codec error: {0}")]
    Codec(#[from] codec::CodecError),
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
}

pub struct PrInfo {
    pub title: String,
    pub author: String,
    pub body: String,
    /// e.g. "owner/repo"
    pub repo: String,
    /// PR number
    pub number: u64,
    /// HEAD commit SHA (for creating review comments)
    pub head_sha: String,
}

fn run_gh(args: &[&str]) -> Result<String, GithubError> {
    let output = Command::new("gh")
        .args(args)
        .output()
        .map_err(|_| GithubError::GhNotFound)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GithubError::GhFailed(stderr.to_string()));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Parse a GitHub PR URL into (owner/repo, number).
///
/// Accepts formats like:
/// - `https://github.com/owner/repo/pull/123`
/// - `github.com/owner/repo/pull/123`
pub fn parse_pr_url(url: &str) -> Result<(String, u64), GithubError> {
    let path = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);
    let path = path.strip_prefix("github.com/").ok_or_else(|| {
        GithubError::InvalidPrUrl(url.to_string())
    })?;

    // Expected: owner/repo/pull/123
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() < 4 || parts[2] != "pull" {
        return Err(GithubError::InvalidPrUrl(url.to_string()));
    }

    let repo = format!("{}/{}", parts[0], parts[1]);
    let number: u64 = parts[3]
        .parse()
        .map_err(|_| GithubError::InvalidPrUrl(url.to_string()))?;

    Ok((repo, number))
}

/// Fetch PR info and diff using the gh CLI.
pub fn fetch_pr(url: &str) -> Result<(PrInfo, String), GithubError> {
    let (repo, number) = parse_pr_url(url)?;

    // Fetch PR metadata as JSON
    let json_str = run_gh(&[
        "pr", "view", url,
        "--json", "title,author,body,headRefOid",
    ])?;

    let json: serde_json::Value = serde_json::from_str(&json_str)?;
    let title = json["title"].as_str().unwrap_or("Untitled PR").to_string();
    let author = json["author"]["login"]
        .as_str()
        .unwrap_or(json["author"]["name"].as_str().unwrap_or("unknown"))
        .to_string();
    let body = json["body"].as_str().unwrap_or("").to_string();
    let head_sha = json["headRefOid"].as_str().unwrap_or("").to_string();

    // Fetch diff
    let diff = run_gh(&["pr", "diff", url])?;

    Ok((
        PrInfo {
            title,
            author,
            body,
            repo,
            number,
            head_sha,
        },
        diff,
    ))
}

/// Fetch review comments (line-level) for a PR.
pub fn fetch_review_comments(repo: &str, number: u64) -> Result<Vec<ReviewComment>, GithubError> {
    let endpoint = format!("repos/{repo}/pulls/{number}/comments");
    let json_str = run_gh(&["api", "--paginate", &endpoint])?;

    // gh api --paginate may return concatenated JSON arrays, so we need to handle that
    let comments: Vec<ReviewComment> = parse_paginated_json(&json_str)?;
    Ok(comments)
}

/// Fetch issue comments (general PR-level) for a PR.
pub fn fetch_issue_comments(repo: &str, number: u64) -> Result<Vec<IssueComment>, GithubError> {
    let endpoint = format!("repos/{repo}/issues/{number}/comments");
    let json_str = run_gh(&["api", "--paginate", &endpoint])?;

    let comments: Vec<IssueComment> = parse_paginated_json(&json_str)?;
    Ok(comments)
}

/// Parse paginated JSON from gh api. When paginating, gh concatenates JSON arrays
/// like `[...][...]`, so we need to handle that.
fn parse_paginated_json<T: serde::de::DeserializeOwned>(json_str: &str) -> Result<Vec<T>, GithubError> {
    let trimmed = json_str.trim();
    if trimmed.is_empty() || trimmed == "[]" {
        return Ok(Vec::new());
    }

    // Try parsing as a single array first
    if let Ok(items) = serde_json::from_str::<Vec<T>>(trimmed) {
        return Ok(items);
    }

    // Handle concatenated arrays: ][
    let mut all_items = Vec::new();
    let mut depth = 0i32;
    let mut start = 0;

    for (i, ch) in trimmed.char_indices() {
        match ch {
            '[' => {
                if depth == 0 {
                    start = i;
                }
                depth += 1;
            }
            ']' => {
                depth -= 1;
                if depth == 0 {
                    let chunk = &trimmed[start..=i];
                    let items: Vec<T> = serde_json::from_str(chunk)?;
                    all_items.extend(items);
                }
            }
            _ => {}
        }
    }

    Ok(all_items)
}

/// Extract encoded storyline data from PR body.
pub fn extract_storyline_from_body(body: &str) -> Result<String, GithubError> {
    codec::extract_from_text(body).map_err(|_| GithubError::NoStoryline)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pr_url() {
        let (repo, num) = parse_pr_url("https://github.com/owner/repo/pull/123").unwrap();
        assert_eq!(repo, "owner/repo");
        assert_eq!(num, 123);

        let (repo, num) = parse_pr_url("github.com/foo/bar/pull/42").unwrap();
        assert_eq!(repo, "foo/bar");
        assert_eq!(num, 42);
    }

    #[test]
    fn test_parse_pr_url_invalid() {
        assert!(parse_pr_url("https://gitlab.com/owner/repo/pull/123").is_err());
        assert!(parse_pr_url("https://github.com/owner/repo/issues/123").is_err());
        assert!(parse_pr_url("not-a-url").is_err());
    }

    #[test]
    fn test_parse_paginated_json() {
        // Single array
        let result: Vec<serde_json::Value> = parse_paginated_json("[1,2,3]").unwrap();
        assert_eq!(result.len(), 3);

        // Concatenated arrays
        let result: Vec<serde_json::Value> = parse_paginated_json("[1,2][3,4]").unwrap();
        assert_eq!(result.len(), 4);

        // Empty
        let result: Vec<serde_json::Value> = parse_paginated_json("").unwrap();
        assert!(result.is_empty());
    }
}
