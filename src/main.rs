use std::fs;
use std::io::{self, Read};
use std::process::Command;

use clap::{Parser, Subcommand};

use diffstory::codec;
use diffstory::comments;
use diffstory::diff_parser;
use diffstory::matcher;
use diffstory::model::Storyline;

#[derive(Parser)]
#[command(name = "diffstory", about = "Organize PR diffs into narrative stories")]
struct Cli {
  #[command(subcommand)]
  command: Commands,
}

#[derive(Subcommand)]
enum Commands {
  /// View a diffstory: from local files or a GitHub PR URL. Generates HTML in /tmp and opens it.
  View {
    /// GitHub PR URL, or omit to use local files
    url: Option<String>,
    /// Path to storyline JSON file (required when not using a URL)
    #[arg(long)]
    story: Option<String>,
    /// Path to diff file (required when not using a URL)
    #[arg(long)]
    diff: Option<String>,
    /// PR title for the viewer header
    #[arg(long)]
    title: Option<String>,
    /// PR author for the viewer header
    #[arg(long)]
    author: Option<String>,
    /// Open the generated HTML in the default browser
    #[arg(long)]
    open: bool,
  },
  /// Encode a storyline JSON to base64-compressed format
  Encode {
    /// Path to storyline JSON file (or - for stdin)
    #[arg(long, default_value = "-")]
    story: String,
    /// Wrap in PR-embeddable HTML format
    #[arg(long)]
    wrap: bool,
  },
  /// Decode a base64-compressed storyline back to JSON
  Decode {
    /// Path to encoded input (or - for stdin)
    #[arg(long, default_value = "-")]
    input: String,
  },
  /// Validate a storyline against a diff
  Validate {
    /// Path to storyline JSON file
    #[arg(long)]
    story: String,
    /// Path to diff file (or - for stdin)
    #[arg(long)]
    diff: Option<String>,
  },
}

fn open_file(path: &std::path::Path) -> io::Result<()> {
  let cmd = if cfg!(target_os = "macos") {
    "open"
  } else if cfg!(target_os = "windows") {
    "start"
  } else {
    "xdg-open"
  };
  Command::new(cmd).arg(path).spawn()?;
  Ok(())
}

fn read_input(path: &str) -> io::Result<String> {
  if path == "-" {
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf)?;
    Ok(buf)
  } else {
    fs::read_to_string(path)
  }
}

fn load_storyline(path: &str) -> Result<Storyline, Box<dyn std::error::Error>> {
  let content = read_input(path)?;
  Ok(serde_json::from_str(&content)?)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let cli = Cli::parse();

  match cli.command {
    Commands::View {
      url,
      story,
      diff,
      title,
      author,
      open,
    } => {
      let html = match url {
        Some(pr_url) => {
          let (pr_info, diff_text) = diffstory::github::fetch_pr(&pr_url)?;
          let encoded = diffstory::github::extract_storyline_from_body(&pr_info.body)?;
          let story = codec::decode(&encoded)?;
          let parsed_diff = diff_parser::parse_diff(&diff_text)?;

          // Fetch comments
          let review_threads = diffstory::github::fetch_review_threads(&pr_info.repo, pr_info.number)
            .unwrap_or_else(|e| {
              eprintln!("warning: failed to fetch review comments: {e}");
              Vec::new()
            });
          let issue_comments = diffstory::github::fetch_issue_comments(&pr_info.repo, pr_info.number)
            .unwrap_or_else(|e| {
              eprintln!("warning: failed to fetch issue comments: {e}");
              Vec::new()
            });

          // Separate bot issue comments
          let (human_issue_comments, bot_issue_comments): (Vec<_>, Vec<_>) =
            issue_comments.into_iter().partition(|c| !c.user.is_bot());

          // Map review threads to hunks, separating resolved/bot
          let (comment_map, outdated, resolved_threads, bot_review_threads) =
            comments::map_threads_to_hunks(review_threads, &parsed_diff);

          let resolved = matcher::resolve_with_comments(
            &story,
            &parsed_diff,
            Some(comment_map),
            human_issue_comments,
            outdated,
            resolved_threads,
            bot_review_threads,
            bot_issue_comments,
          );

          diffstory::html::render(
            &resolved,
            title.as_deref().or(Some(&pr_info.title)),
            author.as_deref().or(Some(&pr_info.author)),
            Some(&pr_info),
          )
        }
        None => {
          let story_path = story
            .ok_or("--story is required when not using a URL")?;
          let diff_path = diff
            .ok_or("--diff is required when not using a URL")?;
          let story = load_storyline(&story_path)?;
          let diff_text = read_input(&diff_path)?;
          let parsed_diff = diff_parser::parse_diff(&diff_text)?;
          let resolved = matcher::resolve(&story, &parsed_diff);
          diffstory::html::render(&resolved, title.as_deref(), author.as_deref(), None)
        }
      };

      let out_path = std::env::temp_dir().join("diffstory.html");
      fs::write(&out_path, &html)?;
      eprintln!("Wrote {}", out_path.display());
      if open {
        open_file(&out_path)?;
      }
    }
    Commands::Encode { story: story_path, wrap } => {
      let story = load_storyline(&story_path)?;
      let encoded = codec::encode(&story)?;
      if wrap {
        println!("{}", codec::wrap(&encoded));
      } else {
        println!("{encoded}");
      }
    }
    Commands::Decode { input } => {
      let content = read_input(&input)?;
      // Try to extract from wrapped format first, fall back to raw
      let encoded = codec::extract_from_text(&content).unwrap_or_else(|_| content.trim().to_string());
      let story = codec::decode(&encoded)?;
      println!("{}", serde_json::to_string_pretty(&story)?);
    }
    Commands::Validate { story: story_path, diff } => {
      let story = load_storyline(&story_path)?;
      match diff {
        Some(diff_path) => {
          let diff_text = read_input(&diff_path)?;
          let parsed_diff = diff_parser::parse_diff(&diff_text)?;
          let result = matcher::validate(&story, &parsed_diff);

          for w in &result.warnings {
            eprintln!("warning: {w}");
          }

          println!(
            "Coverage: {:.0}% ({}/{} hunks)",
            result.coverage_pct(),
            result.covered_hunks,
            result.total_hunks
          );
          if result.uncategorized_hunks > 0 {
            println!("{} uncategorized hunks", result.uncategorized_hunks);
          }
          println!("{} chapters", story.chapters.len());
          println!("{} misc chapters", story.misc.len());
        }
        None => {
          // Just validate JSON structure
          println!("Storyline is valid JSON");
          println!("{} chapters", story.chapters.len());
          let total_refs: usize = story.chapters.iter().map(|c| c.hunks.len()).sum();
          println!("{total_refs} hunk references");
          println!("{} misc chapters", story.misc.len());
        }
      }
    }
  }

  Ok(())
}
