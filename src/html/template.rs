use comrak::{markdown_to_html, Options};

use crate::diff_parser::{DiffLine, FileDiff, Hunk};
use crate::matcher::{IrrelevantGroup, ResolvedChapter, ResolvedHunk, ResolvedStory, UncategorizedHunk};

const TEMPLATE: &str = include_str!("../../assets/template.html");
const CSS: &str = include_str!("../../assets/viewer.css");
const JS: &str = include_str!("../../assets/viewer.js");

pub fn render(story: &ResolvedStory, title: Option<&str>, author: Option<&str>) -> String {
  let display_title = title.unwrap_or("Diffstory");

  let header_author = match author {
    Some(a) => format!("<p class=\"author\">by {}</p>", html_escape(a)),
    None => String::new(),
  };

  let description = match &story.description {
    Some(desc) => format!("<div class=\"story-description\">{}</div>", md_to_html(desc)),
    None => String::new(),
  };

  let toc = render_toc(&story.chapters, &story.irrelevant_groups, &story.uncategorized);
  let chapters = render_chapters(&story.chapters);
  let irrelevant = render_irrelevant(&story.irrelevant_groups);
  let uncategorized = render_uncategorized(&story.uncategorized);
  let (coverage, sidebar_coverage) = render_coverage(story);

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
    .replace("{{CHAPTERS}}", &chapters)
    .replace("{{IRRELEVANT}}", &irrelevant)
    .replace("{{UNCATEGORIZED}}", &uncategorized)
}

fn render_coverage(story: &ResolvedStory) -> (String, String) {
  let chapter_hunks: usize = story.chapters.iter().map(|c| c.hunks.len()).sum();
  let irrelevant_hunks: usize = story.irrelevant_groups.iter().map(|g| g.hunks.len()).sum();
  let covered = chapter_hunks + irrelevant_hunks;
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
  irrelevant: &[IrrelevantGroup],
  uncategorized: &[UncategorizedHunk],
) -> String {
  let mut html = String::new();

  for (i, ch) in chapters.iter().enumerate() {
    html.push_str(&format!(
      "<li><a href=\"#chapter-{i}\" data-chapter=\"{i}\">{}</a></li>\n",
      html_escape(&ch.title)
    ));
  }

  if !irrelevant.is_empty() || !uncategorized.is_empty() {
    html.push_str("<li class=\"toc-section\">Other</li>\n");
  }
  if !irrelevant.is_empty() {
    html.push_str("<li><a href=\"#irrelevant\">Irrelevant</a></li>\n");
  }
  if !uncategorized.is_empty() {
    html.push_str("<li><a href=\"#uncategorized\">Uncategorized</a></li>\n");
  }

  html
}

fn render_chapters(chapters: &[ResolvedChapter]) -> String {
  let mut html = String::new();

  for (i, ch) in chapters.iter().enumerate() {
    html.push_str(&format!("<section class=\"chapter\">\n"));
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

    for resolved_hunk in &ch.hunks {
      html.push_str(&render_resolved_hunk(resolved_hunk));
    }

    html.push_str("</section>\n");
  }

  html
}

fn render_resolved_hunk(rh: &ResolvedHunk) -> String {
  let mut html = String::new();
  html.push_str("<div class=\"diff-file\">\n");
  html.push_str(&render_file_header(&rh.file_diff, &rh.file_path));

  if let Some(note) = &rh.note {
    html.push_str(&format!("<div class=\"hunk-note\">{}</div>\n", md_to_html(note)));
  }

  html.push_str(&render_hunk_table(&rh.hunk));
  html.push_str("</div>\n");
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

fn render_hunk_table(hunk: &Hunk) -> String {
  let mut html = String::new();
  html.push_str("<table class=\"diff-table\">\n");

  // Hunk header row
  html.push_str("<tr class=\"diff-hunk-header\">");
  html.push_str(&format!("<td colspan=\"2\">{}</td>", html_escape(&hunk.header)));
  html.push_str("</tr>\n");

  for line in &hunk.lines {
    let (class, marker, content) = match line {
      DiffLine::Addition(s) => ("diff-line-add", "+", s.as_str()),
      DiffLine::Deletion(s) => ("diff-line-del", "-", s.as_str()),
      DiffLine::Context(s) => ("diff-line-ctx", " ", s.as_str()),
      DiffLine::NoNewlineAtEof => ("diff-line-noeof", "", "\\ No newline at end of file"),
    };
    html.push_str(&format!(
      "<tr class=\"{class}\"><td class=\"diff-marker\">{marker}</td><td class=\"diff-code\">{}</td></tr>\n",
      html_escape(content)
    ));
  }

  html.push_str("</table>\n");
  html
}

fn render_irrelevant(groups: &[IrrelevantGroup]) -> String {
  if groups.is_empty() {
    return String::new();
  }

  let mut html = String::new();
  html.push_str("<div class=\"collapsible\" id=\"irrelevant\">\n");
  html.push_str("<div class=\"collapsible-header\">Irrelevant Changes</div>\n");
  html.push_str("<div class=\"collapsible-body\">\n");

  for group in groups {
    let reason_label = group.reason.as_deref().unwrap_or("No reason given");
    html.push_str("<div class=\"irrelevant-reason\">\n");
    html.push_str(&format!(
      "<div class=\"irrelevant-reason-label\">{}</div>\n",
      html_escape(reason_label)
    ));
    for rh in &group.hunks {
      html.push_str(&render_resolved_hunk(rh));
    }
    html.push_str("</div>\n");
  }

  html.push_str("</div>\n</div>\n");
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

  for uh in uncategorized {
    html.push_str("<div class=\"diff-file\">\n");
    html.push_str(&render_file_header(&uh.file_diff, &uh.file_path));
    html.push_str(&render_hunk_table(&uh.hunk));
    html.push_str("</div>\n");
  }

  html.push_str("</div>\n</div>\n");
  html
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
