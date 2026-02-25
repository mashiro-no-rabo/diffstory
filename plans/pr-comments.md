# GitHub PR Comments Integration

## Context

Diffstory generates narrative HTML viewers from PR diffs, but currently ignores the conversation happening on the PR. This feature integrates GitHub PR comments into the viewer — both displaying existing comments inline and enabling new comment creation via an "export gh commands" workflow that bridges the static HTML with the CLI.

## Scope

- **Show existing comments**: Always fetch when viewing a PR URL. Both line-level review comments (inline with hunks) and general issue comments (separate section). Add a toggle in the viewer toolbar to show/hide.
- **Create new comments**: Clickable diff lines open an inline comment form. An "Export" button copies the equivalent `gh api` command to clipboard for the user to run.

## Implementation

### 1. New module: `src/comments.rs`

Types for GitHub review comments and issue comments:

```rust
// Deserialized from gh api
struct ReviewComment { id, path, line, original_line, side, body, user, created_at, in_reply_to_id }
struct IssueComment { id, body, user, created_at }
struct CommentUser { login }

// After mapping to hunks
struct MappedComment { comment: ReviewComment, line_offset: usize, is_outdated: bool }
struct CommentThread { root: MappedComment, replies: Vec<ReviewComment> }
type CommentMap = HashMap<(String, usize), Vec<CommentThread>>

// Comments that couldn't be mapped to any current hunk
struct OutdatedComment { comment: ReviewComment, file: String }
```

Core functions:
- `map_comments_to_hunks(comments: Vec<ReviewComment>, diff: &ParsedDiff) -> (CommentMap, Vec<OutdatedComment>)` — mapping strategy:
  1. If `line` is present → map to current hunk normally, `is_outdated = false`
  2. If `line` is null but `original_line` exists → try mapping `original_line` against hunk ranges. If it lands in a hunk, show inline with `is_outdated = true` badge. If not, collect as `OutdatedComment`.
  3. If neither maps → collect as `OutdatedComment`
- `thread_comments(...)` — groups replies under root comments by `in_reply_to_id`

Register in `src/lib.rs`.

### 2. Extend `src/diff_parser.rs`

Add `parse_hunk_header(header: &str) -> Option<(u32, u32, u32, u32)>` to extract `(old_start, old_count, new_start, new_count)` from `@@ -a,b +c,d @@` headers. Needed for mapping comment line numbers to hunk offsets.

### 3. Extend `src/github.rs`

Add `parse_pr_url(url) -> Result<(repo, number)>` helper (extract owner/repo and PR number).

Add fetch functions:
- `fetch_review_comments(url) -> Vec<ReviewComment>` via `gh api --paginate repos/{repo}/pulls/{n}/comments`
- `fetch_issue_comments(url) -> Vec<IssueComment>` via `gh api --paginate repos/{repo}/issues/{n}/comments`

Extend `PrInfo` with `url`, `repo`, `number`, and `head_sha` fields (head SHA needed for comment creation commands).

Update `fetch_pr` to populate these fields (add `headRefOid` to the `--json` query).

### 4. Extend `src/matcher.rs`

Add `comments: Vec<CommentThread>` field to `ResolvedHunk` and `UncategorizedHunk`.

Add `resolve_with_comments(storyline, diff, comments: Option<CommentMap>) -> ResolvedStory` — distributes mapped comments to resolved hunks by matching `(file_path, hunk_index)` keys.

Add `issue_comments: Vec<IssueComment>` and `outdated_comments: Vec<OutdatedComment>` fields to `ResolvedStory`.

### 5. Update `src/html/template.rs`

**Comment rendering functions:**
- `render_comment_thread(thread)` — renders a thread as a `<tr class="comment-row">` with root + replies
- `render_single_comment(comment)` — author, date, markdown body (via `md_to_html`)
- `render_issue_comments(comments)` — renders general comments in a dedicated section before chapters
- `render_outdated_comments(comments)` — renders unmappable comments in a collapsible "Outdated Comments" section (grouped by file), placed after misc but before uncategorized hunks

**Update existing functions:**
- `render_hunk_table` — after each diff line, insert comment rows at matching `line_offset`. Also add `data-file` and `data-line` attributes to diff line `<tr>` elements for the JS click handler.
- `render_hunks_grouped`, `render_chapters`, `render_misc`, `render_uncategorized` — thread `comments` through to `render_hunk_table`

**New comment form:**
- Add a hidden `<template id="comment-form-tpl">` with a textarea and "Copy gh command" / "Cancel" buttons
- Pass `pr_url`, `head_sha` as `data-*` attributes on a root element so JS can construct the `gh api` command

**Template placeholders:**
- `{{ISSUE_COMMENTS}}` — general PR comments section
- `{{OUTDATED_COMMENTS}}` — outdated/unmappable review comments section
- `{{PR_META}}` — `data-pr-repo`, `data-pr-number`, `data-pr-head-sha` for JS

**Toolbar:**
- Add a third toggle button for comments visibility (alongside theme and split view toggles)

### 6. Update `assets/viewer.css`

**Comment components:**
```
.comment-thread          — container with subtle background, left border accent
.comment                 — individual comment with header + body
.comment-header          — flex row: author (bold) + relative date (muted)
.comment-body            — rendered markdown, sans-serif font
.comment .outdated-badge — small "outdated" pill badge on inline comments mapped via original_line
.comment-form            — inline textarea + buttons when creating new comment
.issue-comments          — section for general PR comments
.issue-comment           — individual issue comment card
```

**Visibility toggle:**
```
html:not(.show-comments) .comment-row,
html:not(.show-comments) .issue-comments { display: none; }
```

Dark mode variants using existing CSS variables (`--bg-subtle`, `--border`, `--fg-muted`).

### 7. Update `assets/viewer.js`

**Comments visibility toggle:**
- New toolbar button `#comments-toggle` in sidebar toolbar (alongside theme/split)
- Toggles `html.show-comments` class (default: on)
- Persists to `localStorage` as `diffstory-comments`

**New comment creation:**
- Click handler on diff line numbers — clicking a line number inserts the comment form template after that row
- "Copy gh command" button builds: `gh api --method POST repos/{repo}/pulls/{number}/comments -f path={file} -f line={line} -f body='{text}' -f commit_id={sha} -f side=RIGHT`
- Copies to clipboard via `navigator.clipboard.writeText()`, shows brief "Copied!" feedback
- "Reset" button clears the draft and removes the form

**Draft persistence:**
- Auto-save draft text to `localStorage` on input, keyed as `diffstory-draft-{file}-{line}`
- On page load, restore any saved drafts: re-open their comment forms and populate the textarea
- Clear the stored draft when user clicks "Reset" or successfully copies the `gh` command
- Drafts persist indefinitely until explicitly reset or exported

**Reply to existing thread:**
- "Reply" link on each thread expands a reply form
- Builds: `gh api --method POST repos/{repo}/pulls/{number}/comments -f body='{text}' -F in_reply_to={comment_id}`
- Reply drafts also persisted, keyed as `diffstory-reply-{comment_id}`

**Split view update:**
- `generateSplitTables` must preserve `.comment-row` elements when rebuilding tables

### 8. Update `src/main.rs`

In the `View` URL branch:
- Always fetch review comments and issue comments alongside the PR
- Pass them through to `resolve_with_comments`
- Pass `PrInfo` metadata to `html::render` for the JS data attributes

Update `html::render` signature to accept optional `PrInfo` (None for local file mode) and issue comments.

No new subcommand needed — comment creation is handled in the HTML viewer.

## Files modified

| File | Change |
|---|---|
| `src/lib.rs` | Add `pub mod comments` |
| `src/comments.rs` | **New** — types, mapping, threading |
| `src/github.rs` | Add `parse_pr_url`, `fetch_review_comments`, `fetch_issue_comments`, extend `PrInfo` and `fetch_pr` |
| `src/matcher.rs` | Add `comments` to resolved types, `resolve_with_comments`, issue/unattached comments to `ResolvedStory` |
| `src/html/mod.rs` | Update `render` signature |
| `src/html/template.rs` | Comment rendering, issue comments section, inline form template, PR metadata attributes, toolbar toggle |
| `assets/viewer.css` | Comment styles, toggle visibility, dark mode |
| `assets/viewer.js` | Comments toggle, click-to-comment, clipboard export, reply, split view compat |
| `assets/template.html` | New placeholders, comment form template, toolbar button |
| `src/main.rs` | Fetch comments in view URL path, pass to render |

## Verification

1. `cargo build` — compiles cleanly
2. `cargo test` — all 14 tests pass (4 new in comments, 3 new in github)
3. Manual test with a real PR that has review comments:
   ```
   cargo run -- view https://github.com/owner/repo/pull/123 --open
   ```
   - Verify review comments appear inline at correct lines
   - Verify issue comments appear in dedicated section
   - Toggle comments on/off via toolbar button
   - Click a diff line → comment form appears
   - Type text, click "Copy gh command" → verify clipboard contains correct `gh api` command
   - Paste and run the command → verify comment appears on GitHub
4. Test with local files (no PR URL) — comments features gracefully absent
5. Test toggle persistence — reload page, verify comments toggle state persists
