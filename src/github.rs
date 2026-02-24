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
    #[error("codec error: {0}")]
    Codec(#[from] codec::CodecError),
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
}

pub struct PrInfo {
    pub title: String,
    pub author: String,
    pub body: String,
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

/// Fetch PR info and diff using the gh CLI.
pub fn fetch_pr(url: &str) -> Result<(PrInfo, String), GithubError> {
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

    Ok((PrInfo { title, author, body }, diff))
}

/// Extract encoded storyline data from PR body.
pub fn extract_storyline_from_body(body: &str) -> Result<String, GithubError> {
    codec::extract_from_text(body).map_err(|_| GithubError::NoStoryline)
}
