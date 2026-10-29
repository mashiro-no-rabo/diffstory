use std::process::Command;

use thiserror::Error;

use crate::codec;

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
}

fn run_gh(args: &[&str]) -> Result<String, GithubError> {
    let output = Command::new("gh")
        .args(args)
        .env("GH_PAGER", "")
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
        "--json", "title,author,body",
    ])?;

    let json: serde_json::Value = serde_json::from_str(&json_str)?;
    let title = json["title"].as_str().unwrap_or("Untitled PR").to_string();
    let author = json["author"]["login"]
        .as_str()
        .unwrap_or(json["author"]["name"].as_str().unwrap_or("unknown"))
        .to_string();
    let body = json["body"].as_str().unwrap_or("").to_string();

    // Fetch diff
    let diff = run_gh(&["pr", "diff", url])?;

    Ok((
        PrInfo {
            title,
            author,
            body,
            repo,
            number,
        },
        diff,
    ))
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
}
