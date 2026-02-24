# diffstory

Turn PR diffs into guided narratives. Organize hunks into chapters with descriptions and notes, then generate a standalone HTML viewer for reviewers.

## Install

```
cargo install --path .
```

## Usage

**Generate an HTML viewer from a storyline and diff:**
```
diffstory generate --storyline story.json --diff changes.diff -o review.html
```

**Extract from a GitHub PR** (requires `GITHUB_TOKEN`):
```
diffstory extract https://github.com/owner/repo/pull/123 -o review.html
```

**Encode a storyline for embedding in a PR description:**
```
diffstory encode --storyline story.json --wrap
```

**Decode back to JSON:**
```
diffstory decode < encoded.txt
```

**Validate coverage:**
```
diffstory validate --storyline story.json --diff changes.diff
```

## Storyline Format

```json
{
  "description": "Overall reading guide (markdown)",
  "chapters": [
    {
      "title": "Chapter name",
      "description": "What this chapter covers (markdown)",
      "hunks": [
        { "file": "src/main.rs", "hunk_index": 0, "note": "Inline annotation" }
      ]
    }
  ],
  "irrelevant": [
    { "file": "README.md", "hunk_index": 0, "reason": "Routine update" }
  ]
}
```

Hunks are referenced by file path and 0-based index within that file's diff. Every hunk should be assigned to a chapter or marked irrelevant. Unassigned hunks appear in an "Uncategorized" section in the viewer.
