# Diffstory - Narrative Diff Viewer

## Context

PR diffs are flat lists of file changes - hard for reviewers to follow the author's intent. Diffstory lets PR authors organize diff hunks into a guided narrative (chapters with descriptions), encode it as base64 JSON in the PR description, and lets reviewers generate a standalone HTML viewer that presents changes as a structured story.

## Data Model (`src/model.rs`)

The **Storyline** is the central structure. No version field (internal tool). No metadata (PR has title/author/refs). Format is designed to be easy for a Claude skill to build incrementally.

```
Storyline
  ├── description: Option<String> (overall reading guide, markdown)
  ├── chapters: [Chapter]
  │     ├── title: String
  │     ├── description: Option<String> (markdown)
  │     └── hunks: [HunkRef]
  │           ├── file: String (repo-relative path)
  │           ├── hunk_index: usize (0-based index within file's diff)
  │           └── note: Option<String> (inline annotation)
  └── irrelevant: [IrrelevantHunk]
        ├── file: String
        ├── hunk_index: usize
        └── reason: Option<String> (e.g. "mass rename", "generated code")
```

**Hunk selection**: v1 uses simple `file + hunk_index` (0-based within the file's diff). Since a Claude skill generates the storyline from the current diff, index is reliable at creation time. If the diff changes, the skill regenerates. We can add smarter matching (header text, line numbers) later if needed.

**Coverage rules**:
- Each diff hunk belongs to exactly one chapter OR is marked irrelevant. No duplicates.
- `validate` reports coverage %. The story builder skill should prompt until 100%.
- The HTML viewer always shows ALL hunks. Uncovered hunks go in a final "Uncategorized" section.

## Diff Parser (`src/diff_parser.rs`)

Custom unified diff parser. Handles: `diff --git` headers, `---`/`+++` file headers, `@@` hunk headers with context labels, rename detection, binary file markers, no-newline-at-EOF.

Output: `ParsedDiff → Vec<FileDiff> → Vec<Hunk> → Vec<DiffLine>`

## Matcher (`src/matcher.rs`)

Resolves `HunkRef`s against parsed diff. Produces `ResolvedStory`:
- Resolved chapters with actual hunk content
- Resolved irrelevant hunks grouped by reason
- Uncategorized hunks (unreferenced)
- Warnings for out-of-bounds or unresolvable references

## Codec (`src/codec.rs`)

Pipeline: `Storyline → JSON → gzip → base64`

PR embedding:
```html
<details><summary>diffstory</summary>

<!--diffstory:H4sIAAAA...base64...-->

</details>
```

HTML comment makes data invisible on GitHub. Extraction finds `<!--diffstory:` marker.

## HTML Viewer (`src/html/`)

Standalone HTML file, all CSS/JS inlined. Layout:
- **Header**: PR title + author (CLI args or from GitHub API)
- **Sidebar** (fixed): table of contents with chapter links
- **Main content** (scrollable): chapters → descriptions → file diffs with inline notes
- **Footer sections**: Irrelevant (collapsed, grouped by reason) and Uncategorized (collapsed)

**CSS**: GitHub Primer-inspired light theme. Uses Primer color tokens directly:
- Addition bg: `#dafbe1`, deletion bg: `#ffebe9`, hunk header bg: `#ddf4ff`
- Font: `-apple-system, BlinkMacSystemFont, "Segoe UI", ...` (Primer font stack)
- Monospace for code: `ui-monospace, SFMono-Regular, Menlo, monospace`
- **TODO**: dark theme toggle later

**JS** (vanilla, no framework): TOC highlight via IntersectionObserver, collapse/expand, keyboard nav (`j`/`k`).

Assets in `assets/` embedded at compile time via `include_str!`.

## GitHub Integration (`src/github.rs`, feature-gated)

Fetches PR description + diff via GitHub API (`GITHUB_TOKEN` env var). Uses `reqwest::blocking`.

## CLI (`src/main.rs`)

```
diffstory generate --storyline <path> [--diff <path>|-] [--title <str>] [--author <str>] [-o output.html]
diffstory extract <PR-URL> [-o output.html]
diffstory encode [--storyline <path>|-] [--wrap]
diffstory decode [--input <path>|-]
diffstory validate --storyline <path> [--diff <path>]
```

## Project Structure

```
diffstory/
  Cargo.toml
  src/
    main.rs           # clap CLI entry point
    lib.rs            # re-exports
    model.rs          # Storyline, Chapter, HunkRef, IrrelevantHunk
    diff_parser.rs    # unified diff → structured types
    matcher.rs        # HunkRef resolution against parsed diff
    codec.rs          # gzip + base64 encode/decode
    github.rs         # GitHub API (feature-gated)
    html/
      mod.rs          # HTML generation orchestrator
      template.rs     # template interpolation with include_str!
  assets/
    viewer.css        # GitHub-style diff CSS
    viewer.js         # TOC, collapse, keyboard nav
    template.html     # HTML skeleton
  tests/
    fixtures/         # sample diffs + storylines
```

## Dependencies

- `clap` (derive) - CLI
- `serde` + `serde_json` - serialization
- `base64` - encoding
- `flate2` - gzip
- `comrak` - markdown → HTML for descriptions
- `thiserror` - errors
- `reqwest` (blocking, optional) - GitHub API

## Implementation Order

**Phase 1**: `model.rs`, `diff_parser.rs`, `matcher.rs`, `codec.rs` + `encode`/`decode`/`validate` subcommands
**Phase 2**: `html/` module, `assets/`, `generate` subcommand
**Phase 3**: `github.rs`, `extract` subcommand
**Phase 4**: Polish (keyboard nav, split view, dark theme)

## Verification

1. Create sample storyline JSON + sample `.diff` fixture
2. `diffstory validate --storyline sample.json --diff sample.diff` → reports 100% coverage
3. `diffstory generate --storyline sample.json --diff sample.diff -o out.html` → open in browser
4. `diffstory encode --storyline sample.json --wrap` | `diffstory decode` → round-trip
5. Edge cases: missing hunks, renamed files, binary files, partial coverage warnings
