use comrak::{markdown_to_html, Options};

use crate::diff_parser::{DiffLine, FileDiff, Hunk};
use crate::github::PrInfo;
use crate::matcher::{ResolvedGroup, ResolvedHunk, ResolvedSection, ResolvedStory, UncategorizedHunk};

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
    Some(desc) => format!("<div class=\"story-description markdown-body\">{}</div>", md_to_html(desc)),
    None => String::new(),
  };

  let toc = render_toc(&story.groups, &story.uncategorized);
  let groups = render_groups(&story.groups);
  let uncategorized = render_uncategorized(&story.uncategorized);
  let progress = render_progress(story);
  let pr_url = pr_info
    .map(|info| format!("https://github.com/{}/pull/{}", info.repo, info.number))
    .unwrap_or_default();

  TEMPLATE
    .replace("{{TITLE}}", &html_escape(display_title))
    .replace("{{CSS}}", CSS)
    .replace("{{JS}}", JS)
    .replace("{{TOC}}", &toc)
    .replace("{{HEADER_TITLE}}", &html_escape(display_title))
    .replace("{{HEADER_AUTHOR}}", &header_author)
    .replace("{{PROGRESS}}", &progress)
    .replace("{{DESCRIPTION}}", &description)
    .replace("{{GROUPS}}", &groups)
    .replace("{{UNCATEGORIZED}}", &uncategorized)
    .replace("{{PR_URL}}", &html_escape(&pr_url))
    .replace("{{RENDER_ID}}", &render_id())
}

fn render_progress(story: &ResolvedStory) -> String {
  let covered: usize = story
    .groups
    .iter()
    .flat_map(|g| g.sections.iter())
    .map(|s| s.hunks.len())
    .sum();
  let total = covered + story.uncategorized.len();

  if total == 0 {
    return String::new();
  }

  format!(
    "<div class=\"top-progress\">\
      <progress value=\"{covered}\" max=\"{total}\" aria-label=\"Hunks explained\"></progress>\
      <span>{covered} / {total} hunks explained</span>\
    </div>"
  )
}

fn render_toc(groups: &[ResolvedGroup], uncategorized: &[UncategorizedHunk]) -> String {
  let mut html = String::new();
  let mut number = 0;

  for (gi, grp) in groups.iter().enumerate() {
    if grp.sections.is_empty() {
      continue;
    }
    html.push_str(&format!(
      "<li class=\"toc-group\">{}</li>\n",
      html_escape(&grp.title)
    ));
    for (si, sec) in grp.sections.iter().enumerate() {
      let id = section_id(gi, si);
      number += 1;
      html.push_str(&format!(
        "<li><a href=\"#{id}\" data-section=\"{id}\" data-number=\"{number:02}\">{}</a></li>\n",
        html_escape(&sec.title)
      ));
    }
  }

  if !uncategorized.is_empty() {
    html.push_str("<li class=\"toc-group\">Other</li>\n");
    html.push_str(&format!(
      "<li><a href=\"#uncategorized\" data-section=\"uncategorized\" data-number=\"{:02}\">Uncategorized</a></li>\n",
      number + 1
    ));
  }

  html
}

fn section_id(group_idx: usize, section_idx: usize) -> String {
  format!("group-{group_idx}-section-{section_idx}")
}

fn group_id(group_idx: usize) -> String {
  format!("group-{group_idx}")
}

fn render_groups(groups: &[ResolvedGroup]) -> String {
  let mut html = String::new();

  for (gi, grp) in groups.iter().enumerate() {
    if grp.sections.is_empty() {
      continue;
    }

    let grp_id = group_id(gi);
    html.push_str(&format!("<div class=\"story-group\" id=\"{grp_id}\">\n"));
    html.push_str(&format!(
      "<div class=\"story-group-header\"><h2 class=\"story-group-title\">{}</h2>",
      html_escape(&grp.title)
    ));
    if let Some(desc) = &grp.description {
      html.push_str(&format!(
        "<div class=\"story-group-description markdown-body\">{}</div>",
        md_to_html(desc)
      ));
    }
    html.push_str("</div>\n");

    for (si, sec) in grp.sections.iter().enumerate() {
      html.push_str(&render_section(sec, &section_id(gi, si)));
    }

    html.push_str("</div>\n");
  }

  html
}

fn render_section(sec: &ResolvedSection, dom_id: &str) -> String {
  let mut html = String::new();
  html.push_str("<section class=\"story-section\">\n");
  html.push_str(&format!(
    "<div class=\"story-section-header\" id=\"{dom_id}\">\n<h3>{}</h3>\n",
    html_escape(&sec.title)
  ));
  if let Some(desc) = &sec.description {
    html.push_str(&format!(
      "<div class=\"story-section-description markdown-body\">{}</div>\n",
      md_to_html(desc)
    ));
  }
  html.push_str("</div>\n");

  html.push_str(&render_hunks_grouped(&sec.hunks));

  html.push_str("</section>\n");
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
          "<div class=\"hunk-note markdown-body\">{}</div>\n",
          md_to_html(note)
        ));
      }
      html.push_str(&render_hunk_table(&rh.hunk, &rh.file_path));
      i += 1;
    }

    html.push_str("</div>\n");
  }

  html
}

fn render_file_header(file_diff: &FileDiff, path: &str) -> String {
  let mut badges = String::new();
  let mut right_badges = String::new();

  if file_diff.is_rename {
    badges.push_str("<span class=\"badge badge-renamed\">renamed</span>");
  }
  if file_diff.is_binary {
    badges.push_str("<span class=\"badge badge-binary\">binary</span>");
  }
  if file_diff.old_path.is_none() {
    right_badges.push_str("<span class=\"badge badge-new\">new</span>");
  }
  if file_diff.new_path.is_none() {
    badges.push_str("<span class=\"badge badge-deleted\">deleted</span>");
  }

  let display = if file_diff.is_rename {
    format!(
      "{} \u{2192} {}",
      file_diff.old_path.as_deref().unwrap_or("?"),
      file_diff.new_path.as_deref().unwrap_or("?")
    )
  } else {
    path.to_string()
  };

  format!(
    "<div class=\"diff-file-header\" data-file=\"{}\">\
      <button class=\"diff-fold-btn\" type=\"button\" title=\"Toggle fold\" aria-label=\"Toggle fold\"></button>\
      <div class=\"diff-file-header-main\">{badges}<span>{}</span></div>\
      <div class=\"diff-file-actions\">{right_badges}\
        <label class=\"viewed-toggle\" title=\"Mark this file as viewed\">\
          <input type=\"checkbox\" class=\"viewed-checkbox\">Viewed\
        </label>\
      </div>\
    </div>\n",
    html_escape(path),
    html_escape(&display)
  )
}

fn render_hunk_table(hunk: &Hunk, file_path: &str) -> String {
  let mut html = String::new();
  html.push_str(&format!(
    "<div class=\"diff-scroll\"><table class=\"diff-table\" data-file=\"{}\">\n",
    html_escape(file_path)
  ));

  html.push_str("<tr class=\"diff-hunk-header\">");
  html.push_str(&format!("<td colspan=\"3\">{}</td>", html_escape(&hunk.header)));
  html.push_str("</tr>\n");

  let (mut new_line, mut old_line) = parse_hunk_start(&hunk.header);

  for line in &hunk.lines {
    let (class, marker, content, old_ln, new_ln) = match line {
      DiffLine::Addition(s) => {
        new_line += 1;
        ("diff-line-add", "+", s.as_str(), None, Some(new_line - 1))
      }
      DiffLine::Deletion(s) => {
        old_line += 1;
        ("diff-line-del", "-", s.as_str(), Some(old_line - 1), None)
      }
      DiffLine::Context(s) => {
        new_line += 1;
        old_line += 1;
        ("diff-line-ctx", " ", s.as_str(), Some(old_line - 1), Some(new_line - 1))
      }
      DiffLine::NoNewlineAtEof => ("diff-line-noeof", "", "\\ No newline at end of file", None, None),
    };

    let mut line_attrs = String::new();
    if let Some(ln) = old_ln {
      line_attrs.push_str(&format!(" data-old-line=\"{ln}\""));
    }
    if let Some(ln) = new_ln {
      line_attrs.push_str(&format!(" data-new-line=\"{ln}\""));
    }

    html.push_str(&format!(
      "<tr class=\"{class}\"{line_attrs}>\
        <td class=\"diff-line-num\">{}</td>\
        <td class=\"diff-marker\">{marker}</td>\
        <td class=\"diff-code\">{}</td>\
      </tr>\n",
      new_ln.or(old_ln).map(|ln| ln.to_string()).unwrap_or_default(),
      html_escape(content)
    ));
  }

  html.push_str("</table></div>\n");
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
      html.push_str(&render_hunk_table(&uncategorized[i].hunk, &uncategorized[i].file_path));
      i += 1;
    }

    html.push_str("</div>\n");
  }

  html.push_str("</div>\n</div>\n");
  html
}

// Notes are keyed per render so a regenerated viewer doesn't show notes made against an older diff
fn render_id() -> String {
  std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .map(|d| d.as_millis().to_string())
    .unwrap_or_default()
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
