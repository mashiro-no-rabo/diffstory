# Diffstory

Rust CLI tool that organizes PR diff hunks into a narrative with chapters, then renders a standalone HTML viewer.

## Build & Test

```
cargo build
cargo test
```

## Architecture

- `src/model.rs` — Data types: Storyline, Chapter, HunkRef
- `src/diff_parser.rs` — Unified diff parser (git format)
- `src/matcher.rs` — Resolves HunkRefs against parsed diffs, tracks coverage, distributes comments to hunks
- `src/codec.rs` — JSON → gzip → base64 encode/decode, PR embedding format
- `src/comments.rs` — GitHub PR comment types, line-to-hunk mapping, threading
- `src/html/` — Standalone HTML generation with inlined CSS/JS from `assets/`
- `src/github.rs` — GitHub PR fetching via `gh` CLI (metadata, diff, review comments, issue comments)
- `src/main.rs` — clap CLI with subcommands: view, encode, decode, validate

## Conventions

- Storyline JSON uses `file` + `hunk_index` (0-based) to reference diff hunks; `misc` chapters use the same `{title, description, hunks}` structure as regular chapters
- PR embedding uses `<!--diffstory:BASE64-->` inside a `<details>` block
- HTML viewer is fully self-contained (no external dependencies), with dark theme, split view, and comments toggles
- Markdown in descriptions/notes rendered via comrak
- GitHub integration uses `gh` CLI (no API token management needed)
- PR comments (review + issue) are fetched and displayed inline when viewing a PR URL; click diff line numbers to create new comments via exported `gh api` commands

## Test Fixtures

Sample diff and storyline in `tests/fixtures/`. Use for manual testing:
```
cargo run -- validate --story tests/fixtures/sample.json --diff tests/fixtures/sample.diff
cargo run -- view --story tests/fixtures/sample.json --diff tests/fixtures/sample.diff
```
