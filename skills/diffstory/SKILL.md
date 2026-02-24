---
name: diffstory
description: Generate a narrative storyline from the current branch's diff, organizing hunks into chapters for reviewers. Use when you want to create a guided code review story from PR changes.
allowed-tools: Bash, Read, Grep, Glob, Write
---

You are building a **diffstory** — a narrative that organizes a PR's diff hunks into chapters so reviewers can follow the author's intent.

## Step 1: Get the diff

Get the diff for the current branch against the base branch:

```
jj diff --git
```

If that fails, fall back to:
```
git diff main...HEAD
```

Save the diff to a temporary file for later use with `diffstory validate`.

## Step 2: Analyze the diff

Parse the diff and list every file and hunk. For each file, count the hunks (0-indexed). Present a summary like:

```
src/main.rs: 3 hunks (0, 1, 2)
src/lib.rs: 1 hunk (0)
README.md: 1 hunk (0)
```

Read the actual source files to understand what the changes do.

## Step 3: Build the storyline interactively

Propose an initial chapter structure based on your understanding of the changes. Each chapter should group related hunks that tell a coherent part of the story. Present your proposal and ask the user to confirm or adjust.

For each chapter, draft:
- **title**: concise name for this group of changes
- **description**: markdown explanation of what this chapter covers and why
- **hunks**: which file + hunk_index pairs belong here
- **notes**: optional inline annotations for specific hunks that need extra context

Also identify any hunks that are **irrelevant** (generated code, mass renames, formatting-only, etc.) and propose reasons.

## Step 4: Validate coverage

Write the storyline JSON to a file and run:

```
diffstory validate --storyline <path> --diff <diff-path>
```

Every hunk must be assigned to exactly one chapter or marked irrelevant. If coverage is not 100%, identify the missing hunks and ask the user where they belong.

Iterate until validation shows 100% coverage.

## Step 5: Output

Once validated, present two options:

1. **Encode for PR embedding**: `diffstory encode --storyline <path> --wrap` — outputs the `<details>` block to paste into the PR description
2. **View HTML**: `diffstory view --storyline <path> --diff <diff-path>` — generates HTML in /tmp and opens it in the browser

Ask the user which they want (or both).

## Storyline JSON format

```json
{
  "description": "Overall reading guide (markdown)",
  "chapters": [
    {
      "title": "Chapter title",
      "description": "What and why (markdown)",
      "hunks": [
        { "file": "src/main.rs", "hunk_index": 0, "note": "Optional annotation" }
      ]
    }
  ],
  "irrelevant": [
    { "file": "README.md", "hunk_index": 0, "reason": "Formatting only" }
  ]
}
```

## Guidelines

- Group hunks by **logical concern**, not by file. A single file's hunks may span multiple chapters.
- Write descriptions that explain **why**, not just what. Reviewers can see the code — they need the narrative.
- Keep chapter titles short (3-6 words).
- Use notes sparingly — only when a hunk needs context that isn't obvious from the description.
- Order chapters to tell a story: setup before usage, core logic before edge cases.
