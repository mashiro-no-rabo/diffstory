---
name: diffstory
description: Generate a narrative storyline from the current branch's diff, or view an existing diffstory from a PR URL. Use when you want to create or view a guided code review story.
allowed-tools: Bash, Read, Grep, Glob, Write
---

## Viewing an existing diffstory

If the user provides a **GitHub PR URL** and wants to view (not generate) a diffstory, simply run:

```
diffstory view <PR_URL>
```

This fetches the PR diff and embedded storyline, generates the HTML viewer, and opens it. Do NOT proceed with the generation steps below.

---

## Generating a new diffstory

You are building a **diffstory** — a narrative that organizes a PR's diff hunks into chapters so reviewers can follow the author's intent.

## Step 1: Get the diff and check for existing storyline

First, check if the current change has multiple parents (e.g. a merge commit):

```
jj log -r @-
```

If this returns multiple changes, ask the user which parent to diff against before proceeding.

Then get the diff for the current branch against the base branch:

```
jj diff --git -f main
```

If that fails, fall back to:
```
git diff main...HEAD
```

Generate a unique session ID (e.g. using `mktemp -d /tmp/diffstory-XXXXXX`) and save all files under that directory — e.g. `$SESSION/diff.patch` and `$SESSION/story.json`. This allows multiple sessions to work on different diffs in parallel without overwriting each other.

Then check if the PR already has an embedded diffstory by fetching the PR description:

```
gh pr view --json body -q .body
```

If the body contains a `<!--diffstory:` marker inside a `<details>` tag, decode the existing storyline:

```
gh pr view --json body -q .body | diffstory decode
```

Save this as the starting storyline JSON. The existing chapters, descriptions, and notes should be used as the basis — preserve what's there and only adjust for hunks that have changed in the new diff.

## Step 2: Analyze the diff

Parse the diff and list every file and hunk. For each file, count the hunks (0-indexed). Present a summary like:

```
src/main.rs: 3 hunks (0, 1, 2)
src/lib.rs: 1 hunk (0)
README.md: 1 hunk (0)
```

Read the actual source files to understand what the changes do.

## Step 3: Open the HTML viewer

Before building the storyline, offer to open the HTML viewer so the user can see changes live as you edit. Write an initial empty storyline JSON and open it:

```
diffstory view --story <path> --diff <diff-path>
```

This opens `/tmp/diffstory.html` in the browser. Every time you update the storyline JSON and re-run `diffstory view`, the file is overwritten and the user can refresh the browser to see the latest version.

## Step 4: Build the storyline interactively

Propose an initial chapter structure based on your understanding of the changes. Each chapter should group related hunks that tell a coherent part of the story. Present your proposal and ask the user to confirm or adjust.

For each chapter, draft:
- **title**: concise name for this group of changes
- **description**: markdown explanation of what this chapter covers and why
- **hunks**: which file + hunk_index pairs belong here
- **notes**: optional inline annotations for specific hunks that need extra context

Also identify any hunks that belong in **misc** chapters (generated code, mass renames, formatting-only, etc.) — these use the same `{title, description, hunks}` structure as regular chapters but are displayed in a collapsible "Misc" section.

After each round of edits, re-run `diffstory view --story <path> --diff <diff-path>` so the user can refresh the browser and see the updated story.

## Step 5: Validate coverage

Write the updated storyline JSON and run:

```
diffstory validate --story <path> --diff <diff-path>
```

Every hunk must be assigned to exactly one chapter or misc chapter. If coverage is not 100%, identify the missing hunks and ask the user where they belong.

Iterate until validation shows 100% coverage, re-running `diffstory view` after each change.

## Step 6: Embed in PR

Once validated and the user is happy with the HTML preview, ask the user if they want to update the PR description directly using `gh`. If confirmed:

**CRITICAL**: Never load the encoded base64 string into context — it's opaque data. Pipe everything through files:

1. Encode to a temp file: `diffstory encode --story <path> --wrap > $SESSION/encoded.txt`
2. Get current PR body to a temp file: `gh pr view --json body -q .body > $SESSION/pr_body.txt`
3. Check if the body already contains a diffstory `<details>` block (look for `<!--diffstory:` within a `<details>` tag)
4. Do the replacement/append in a script (sed/python) that reads both files and writes the new body to `$SESSION/pr_body_new.txt`:
   - If a `<details>` block with `<!--diffstory:` exists, replace **only** that block with the new encoded output. Do NOT modify any other lines.
   - If not found, append the encoded output to the end of the existing description.
5. Update the PR: `gh pr edit --body-file $SESSION/pr_body_new.txt`

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

## Guidelines

- Group hunks by **logical concern**, not by file. A single file's hunks may span multiple chapters.
- Write descriptions that explain **why**, not just what. Reviewers can see the code — they need the narrative.
- Keep chapter titles short (3-6 words).
- **Avoid per-hunk notes.** A chapter's description should be sufficient to explain all its hunks. Only add a `note` to a hunk when it would be genuinely confusing without one (e.g. a non-obvious side effect, a subtle ordering dependency). Most chapters should have zero notes.
- Order chapters to tell a story: setup before usage, core logic before edge cases.
