use comrak::{markdown_to_html, Options};

use crate::comments::{CommentThread, IssueComment, OutdatedComment, ReviewComment};
use crate::diff_parser::{DiffLine, FileDiff, Hunk};
use crate::github::PrInfo;
use crate::matcher::{ResolvedChapter, ResolvedHunk, ResolvedStory, UncategorizedHunk};

const TEMPLATE: &str = include_str!("../../assets/template.html");
const CSS: &str = include_str!("../../assets/viewer.css");
const JS: &str = include_str!("../../assets/viewer.js");

pub fn render(story: &ResolvedStory, title: Option<&str>, author: Option<&str>, pr_info: Option<&PrInfo>) -> String {
  let display_title = title.unwrap_or("Diffstory");

  let header_author = match author {
    Some(a) => format!("<p class=\"author\">by {}</p>", html_escape(a)),
    None => String::new(),
  };

  let description = match &story.description {
    Some(desc) => format!("<div class=\"story-description\">{}</div>", md_to_html(desc)),
    None => String::new(),
  };

  let toc = render_toc(&story.chapters, &story.misc, &story.uncategorized);
  let chapters = render_chapters(&story.chapters);
  let misc = render_misc(&story.misc);
  let uncategorized = render_uncategorized(&story.uncategorized);
  let (coverage, sidebar_coverage) = render_coverage(story);
  let issue_comments = render_issue_comments(&story.issue_comments);
  let outdated_comments = render_outdated_comments(&story.outdated_comments);
  let pr_meta = render_pr_meta(pr_info);
  let has_comments = pr_info.is_some();

  TEMPLATE
    .replace("{{TITLE}}", &html_escape(display_title))
    .replace("{{CSS}}", CSS)
    .replace("{{JS}}", JS)
    .replace("{{TOC}}", &toc)
    .replace("{{HEADER_TITLE}}", &html_escape(display_title))
    .replace("{{HEADER_AUTHOR}}", &header_author)
    .replace("{{COVERAGE}}", &coverage)
    .replace("{{SIDEBAR_COVERAGE}}", &sidebar_coverage)
    .replace("{{DESCRIPTION}}", &description)
    .replace("{{ISSUE_COMMENTS}}", &issue_comments)
    .replace("{{CHAPTERS}}", &chapters)
    .replace("{{MISC}}", &misc)
    .replace("{{OUTDATED_COMMENTS}}", &outdated_comments)
    .replace("{{UNCATEGORIZED}}", &uncategorized)
    .replace("{{PR_META}}", &pr_meta)
    .replace("{{COMMENTS_TOGGLE}}", if has_comments {
      "<button class=\"toolbar-btn\" id=\"comments-toggle\" title=\"Toggle comments\">\
        <span class=\"icon-comments-on\">&#128172;</span><span class=\"icon-comments-off\">&#128173;</span>\
      </button>"
    } else { "" })
    .replace("{{EXPORT_BTN}}", if has_comments {
      "<button class=\"toolbar-btn\" id=\"export-comments\" title=\"Export all draft comments\">&#128230;</button>"
    } else { "" })
}

fn render_pr_meta(pr_info: Option<&PrInfo>) -> String {
  match pr_info {
    Some(info) => format!(
      "<div id=\"pr-meta\" data-pr-repo=\"{}\" data-pr-number=\"{}\" data-pr-head-sha=\"{}\" style=\"display:none\"></div>",
      html_escape(&info.repo),
      info.number,
      html_escape(&info.head_sha),
    ),
    None => String::new(),
  }
}

fn render_coverage(story: &ResolvedStory) -> (String, String) {
  let chapter_hunks: usize = story.chapters.iter().map(|c| c.hunks.len()).sum();
  let misc_hunks: usize = story.misc.iter().map(|c| c.hunks.len()).sum();
  let covered = chapter_hunks + misc_hunks;
  let total = covered + story.uncategorized.len();

  if total == 0 {
    return (String::new(), String::new());
  }

  let pct = (covered as f64 / total as f64) * 100.0;
  let cls = if story.uncategorized.is_empty() { "full" } else { "partial" };

  let inner = format!(
    "<div class=\"coverage\">\
      <div class=\"coverage-bar\"><div class=\"coverage-fill {cls}\" style=\"width:{pct:.0}%\"></div></div>\
      <span>{covered}/{total} hunks covered ({pct:.0}%)</span>\
    </div>"
  );

  let sidebar = format!("<div class=\"sidebar-coverage\">{inner}</div>");
  (inner, sidebar)
}

fn render_toc(
  chapters: &[ResolvedChapter],
  misc: &[ResolvedChapter],
  uncategorized: &[UncategorizedHunk],
) -> String {
  let mut html = String::new();

  for (i, ch) in chapters.iter().enumerate() {
    html.push_str(&format!(
      "<li><a href=\"#chapter-{i}\" data-chapter=\"{i}\">{}</a></li>\n",
      html_escape(&ch.title)
    ));
  }

  if !misc.is_empty() {
    html.push_str("<li class=\"toc-section\">Misc</li>\n");
    for (i, ch) in misc.iter().enumerate() {
      html.push_str(&format!(
        "<li><a href=\"#misc-{i}\" data-chapter=\"misc-{i}\">{}</a></li>\n",
        html_escape(&ch.title)
      ));
    }
  }
  if !uncategorized.is_empty() {
    html.push_str("<li class=\"toc-section\">Other</li>\n");
    html.push_str("<li><a href=\"#uncategorized\">Uncategorized</a></li>\n");
  }

  html
}

fn render_chapters(chapters: &[ResolvedChapter]) -> String {
  let mut html = String::new();

  for (i, ch) in chapters.iter().enumerate() {
    html.push_str("<section class=\"chapter\">\n");
    html.push_str(&format!(
      "<div class=\"chapter-header\" id=\"chapter-{i}\">\n<h2>{}</h2>\n",
      html_escape(&ch.title)
    ));
    if let Some(desc) = &ch.description {
      html.push_str(&format!(
        "<div class=\"chapter-description\">{}</div>\n",
        md_to_html(desc)
      ));
    }
    html.push_str("</div>\n");

    html.push_str(&render_hunks_grouped(&ch.hunks));

    html.push_str("</section>\n");
  }

  html
}

fn render_hunks_grouped(hunks: &[ResolvedHunk]) -> String {
  let mut html = String::new();
  let mut i = 0;

  while i < hunks.len() {
    let file_path = &hunks[i].file_path;
    html.push_str("<div class=\"diff-file\">\n");
    html.push_str(&render_file_header(&hunks[i].file_diff, file_path));

    // Render all consecutive hunks from the same file
    while i < hunks.len() && hunks[i].file_path == *file_path {
      let rh = &hunks[i];
      if let Some(note) = &rh.note {
        html.push_str(&format!(
          "<div class=\"hunk-note\">{}</div>\n",
          md_to_html(note)
        ));
      }
      html.push_str(&render_hunk_table(&rh.hunk, &rh.file_path, &rh.comments));
      i += 1;
    }

    html.push_str("</div>\n");
  }

  html
}

fn render_file_header(file_diff: &FileDiff, path: &str) -> String {
  let mut badges = String::new();

  if file_diff.is_rename {
    badges.push_str("<span class=\"badge badge-renamed\">renamed</span>");
  }
  if file_diff.is_binary {
    badges.push_str("<span class=\"badge badge-binary\">binary</span>");
  }
  if file_diff.old_path.is_none() {
    badges.push_str("<span class=\"badge badge-new\">new</span>");
  }
  if file_diff.new_path.is_none() {
    badges.push_str("<span class=\"badge badge-deleted\">deleted</span>");
  }

  let display = if file_diff.is_rename {
    format!(
      "{} → {}",
      file_diff.old_path.as_deref().unwrap_or("?"),
      file_diff.new_path.as_deref().unwrap_or("?")
    )
  } else {
    path.to_string()
  };

  format!(
    "<div class=\"diff-file-header\">{badges}<span>{}</span></div>\n",
    html_escape(&display)
  )
}

fn render_hunk_table(hunk: &Hunk, file_path: &str, comments: &[CommentThread]) -> String {
  let mut html = String::new();
  html.push_str("<table class=\"diff-table\">\n");

  // Hunk header row
  html.push_str("<tr class=\"diff-hunk-header\">");
  html.push_str(&format!("<td colspan=\"3\">{}</td>", html_escape(&hunk.header)));
  html.push_str("</tr>\n");

  // Parse hunk header for line numbers
  let (mut new_line, mut _old_line) = parse_hunk_start(&hunk.header);

  for (offset, line) in hunk.lines.iter().enumerate() {
    let (class, marker, content, cur_new_line) = match line {
      DiffLine::Addition(s) => {
        let ln = new_line;
        new_line += 1;
        ("diff-line-add", "+", s.as_str(), Some(ln))
      }
      DiffLine::Deletion(s) => {
        _old_line += 1;
        ("diff-line-del", "-", s.as_str(), None::<u32>)
      }
      DiffLine::Context(s) => {
        let ln = new_line;
        new_line += 1;
        _old_line += 1;
        ("diff-line-ctx", " ", s.as_str(), Some(ln))
      }
      DiffLine::NoNewlineAtEof => ("diff-line-noeof", "", "\\ No newline at end of file", None),
    };

    // Add data attributes for the comment click handler
    let line_attr = match cur_new_line {
      Some(ln) => format!(" data-file=\"{}\" data-line=\"{}\"", html_escape(file_path), ln),
      None => String::new(),
    };

    html.push_str(&format!(
      "<tr class=\"{class}\"{line_attr}>\
        <td class=\"diff-line-num\">{}</td>\
        <td class=\"diff-marker\">{marker}</td>\
        <td class=\"diff-code\">{}</td>\
      </tr>\n",
      match cur_new_line { Some(ln) => ln.to_string(), None => String::new() },
      html_escape(content)
    ));

    // Insert comment rows at this offset
    for thread in comments.iter().filter(|t| t.root.line_offset == offset) {
      html.push_str(&render_comment_thread(thread));
    }
  }

  html.push_str("</table>\n");
  html
}

fn parse_hunk_start(header: &str) -> (u32, u32) {
  // Parse @@ -old_start,old_count +new_start,new_count @@
  let header = header.strip_prefix("@@ ").unwrap_or(header);
  let end = header.find(" @@").unwrap_or(header.len());
  let range_str = &header[..end];

  let mut parts = range_str.split(' ');
  let old_start = parts.next()
    .and_then(|s| s.strip_prefix('-'))
    .and_then(|s| s.split(',').next())
    .and_then(|s| s.parse::<u32>().ok())
    .unwrap_or(1);
  let new_start = parts.next()
    .and_then(|s| s.strip_prefix('+'))
    .and_then(|s| s.split(',').next())
    .and_then(|s| s.parse::<u32>().ok())
    .unwrap_or(1);

  (new_start, old_start)
}

fn render_comment_thread(thread: &CommentThread) -> String {
  let mut html = String::new();
  html.push_str("<tr class=\"comment-row\"><td colspan=\"3\">\n");
  html.push_str("<div class=\"comment-thread\">\n");

  // Root comment
  html.push_str(&render_single_comment(
    &thread.root.comment,
    thread.root.is_outdated,
  ));

  // Replies
  for reply in &thread.replies {
    html.push_str(&render_single_comment(reply, false));
  }

  // Reply link
  html.push_str(&format!(
    "<div class=\"comment-reply-link\"><a href=\"#\" class=\"reply-btn\" data-comment-id=\"{}\">Reply</a></div>\n",
    thread.root.comment.id
  ));

  html.push_str("</div>\n");
  html.push_str("</td></tr>\n");
  html
}

fn render_single_comment(comment: &ReviewComment, is_outdated: bool) -> String {
  let outdated_badge = if is_outdated {
    " <span class=\"outdated-badge\">outdated</span>"
  } else {
    ""
  };

  format!(
    "<div class=\"comment\">\
      <div class=\"comment-header\">\
        <span class=\"comment-author\">{}</span>{outdated_badge}\
        <span class=\"comment-date\">{}</span>\
      </div>\
      <div class=\"comment-body\">{}</div>\
    </div>\n",
    html_escape(&comment.user.login),
    format_date(&comment.created_at),
    md_to_html(&comment.body),
  )
}

fn render_issue_comments(comments: &[IssueComment]) -> String {
  if comments.is_empty() {
    return String::new();
  }

  let mut html = String::new();
  html.push_str("<section class=\"issue-comments\">\n");
  html.push_str("<h2>Discussion</h2>\n");

  for comment in comments {
    html.push_str(&format!(
      "<div class=\"issue-comment\">\
        <div class=\"comment-header\">\
          <span class=\"comment-author\">{}</span>\
          <span class=\"comment-date\">{}</span>\
        </div>\
        <div class=\"comment-body\">{}</div>\
      </div>\n",
      html_escape(&comment.user.login),
      format_date(&comment.created_at),
      md_to_html(&comment.body),
    ));
  }

  html.push_str("</section>\n");
  html
}

fn render_outdated_comments(comments: &[OutdatedComment]) -> String {
  if comments.is_empty() {
    return String::new();
  }

  let mut html = String::new();
  html.push_str("<div class=\"collapsible\" id=\"outdated-comments\">\n");
  html.push_str(&format!(
    "<div class=\"collapsible-header\">Outdated Comments ({} comments)</div>\n",
    comments.len()
  ));
  html.push_str("<div class=\"collapsible-body\">\n");

  // Group by file
  let mut by_file: Vec<(&str, Vec<&OutdatedComment>)> = Vec::new();
  for c in comments {
    if let Some(group) = by_file.iter_mut().find(|(f, _)| *f == c.file.as_str()) {
      group.1.push(c);
    } else {
      by_file.push((&c.file, vec![c]));
    }
  }

  for (file, group) in &by_file {
    html.push_str(&format!(
      "<div class=\"outdated-file-group\">\
        <div class=\"outdated-file-header\">{}</div>\n",
      html_escape(file)
    ));
    for oc in group {
      html.push_str(&format!(
        "<div class=\"comment\">\
          <div class=\"comment-header\">\
            <span class=\"comment-author\">{}</span>\
            <span class=\"outdated-badge\">outdated</span>\
            <span class=\"comment-date\">{}</span>\
          </div>\
          <div class=\"comment-body\">{}</div>\
        </div>\n",
        html_escape(&oc.comment.user.login),
        format_date(&oc.comment.created_at),
        md_to_html(&oc.comment.body),
      ));
    }
    html.push_str("</div>\n");
  }

  html.push_str("</div>\n</div>\n");
  html
}

fn render_misc(misc: &[ResolvedChapter]) -> String {
  if misc.is_empty() {
    return String::new();
  }

  let mut html = String::new();

  for (i, ch) in misc.iter().enumerate() {
    html.push_str("<section class=\"chapter\">\n");
    html.push_str(&format!(
      "<div class=\"chapter-header\" id=\"misc-{i}\">\n<h2>{}</h2>\n",
      html_escape(&ch.title)
    ));
    if let Some(desc) = &ch.description {
      html.push_str(&format!(
        "<div class=\"chapter-description\">{}</div>\n",
        md_to_html(desc)
      ));
    }
    html.push_str("</div>\n");

    html.push_str(&render_hunks_grouped(&ch.hunks));

    html.push_str("</section>\n");
  }
  html
}

fn render_uncategorized(uncategorized: &[UncategorizedHunk]) -> String {
  if uncategorized.is_empty() {
    return String::new();
  }

  let mut html = String::new();
  html.push_str("<div class=\"collapsible\" id=\"uncategorized\">\n");
  html.push_str(&format!(
    "<div class=\"collapsible-header\">Uncategorized ({} hunks)</div>\n",
    uncategorized.len()
  ));
  html.push_str("<div class=\"collapsible-body\">\n");

  let mut i = 0;
  while i < uncategorized.len() {
    let file_path = &uncategorized[i].file_path;
    html.push_str("<div class=\"diff-file\">\n");
    html.push_str(&render_file_header(&uncategorized[i].file_diff, file_path));

    while i < uncategorized.len() && uncategorized[i].file_path == *file_path {
      html.push_str(&render_hunk_table(&uncategorized[i].hunk, &uncategorized[i].file_path, &uncategorized[i].comments));
      i += 1;
    }

    html.push_str("</div>\n");
  }

  html.push_str("</div>\n</div>\n");
  html
}

/// Format an ISO date string to a shorter display format.
fn format_date(iso: &str) -> String {
  // Just show the date portion: "2024-01-15T10:30:00Z" -> "2024-01-15"
  iso.split('T').next().unwrap_or(iso).to_string()
}

fn md_to_html(markdown: &str) -> String {
  markdown_to_html(markdown, &Options::default())
}

fn html_escape(s: &str) -> String {
  s.replace('&', "&amp;")
    .replace('<', "&lt;")
    .replace('>', "&gt;")
    .replace('"', "&quot;")
}
