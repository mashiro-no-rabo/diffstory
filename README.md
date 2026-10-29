# diffstory

Turn PR diffs into guided narratives. Organize hunks into groups and sections with descriptions and notes, then view them as a standalone HTML story.

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
  "groups": [
    {
      "title": "Main",
      "description": "Optional group-level note (markdown)",
      "sections": [
        {
          "title": "Section name",
          "description": "What this section covers (markdown)",
          "hunks": [
            { "file": "src/main.rs", "hunk_index": 0, "note": "Inline annotation" }
          ]
        }
      ]
    },
    {
      "title": "Misc",
      "sections": [
        {
          "title": "Routine Updates",
          "hunks": [
            { "file": "README.md", "hunk_index": 0 }
          ]
        }
      ]
    }
  ]
}
```

Hunks are referenced by file path and 0-based index within that file's diff. Every hunk should be assigned to a section. Unassigned hunks appear in an "Uncategorized" block in the viewer.

## Review Comments

Select any text in the viewer (story/group/section descriptions, hunk notes, or diff lines) and start typing to open the comment editor. Comments show up beside the section they belong to, and in the **Comments** drawer in the top bar. They are saved in localStorage for the current render; regenerating the viewer starts fresh.

**Copy prompt** (in the drawer) copies all comments as a single prompt for an agent session, then clears them. Text selections are quoted; code selections include `file:Lstart-Lend` (new-file line numbers, or `(old)` for deletion-only selections) and a diff snippet.
