# diffstory

Turn PR diffs into guided narratives. Organize hunks into chapters with descriptions and notes, then view them as a standalone HTML story.

## Install

Use the `/install` skill in Claude Code, which also registers the `/diffstory` skill for generating storylines.

## Usage

**View a GitHub PR's diffstory** (requires `gh` CLI):
```
diffstory view https://github.com/owner/repo/pull/123
```

**View from local files:**
```
diffstory view --story story.json --diff changes.diff
```

Both open a standalone HTML viewer in your browser.

**Encode a storyline for embedding in a PR description:**
```
diffstory encode --story story.json --wrap
```

**Decode back to JSON:**
```
diffstory decode < encoded.txt
```

**Validate coverage:**
```
diffstory validate --story story.json --diff changes.diff
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
  "misc": [
    {
      "title": "Routine Updates",
      "hunks": [
        { "file": "README.md", "hunk_index": 0 }
      ]
    }
  ]
}
```

Hunks are referenced by file path and 0-based index within that file's diff. Every hunk should be assigned to a chapter or a misc chapter. Unassigned hunks appear in an "Uncategorized" section in the viewer.
