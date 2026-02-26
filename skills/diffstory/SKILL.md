---
name: diffstory
description: Generate a narrative storyline from the current branch's diff, or view an existing diffstory from a PR URL. Use when you want to create or view a guided code review story.
allowed-tools: Bash, Read, Grep, Glob, Write, AskUserQuestion
---

## Viewing an existing diffstory

If the user provides a **GitHub PR URL** and wants to view (not generate) a diffstory, simply run:

```
diffstory view --open <PR_URL>
```

This fetches the PR diff and embedded storyline, generates the HTML viewer, and opens it. Review comments and issue comments from the PR are automatically fetched and displayed inline. Do NOT proceed with the generation steps below.

---

## Generating a new diffstory

You are building a **diffstory** — a narrative that organizes a PR's diff hunks into **chapters**, grouped under one or more **sections**, so reviewers can follow the author's intent.

A storyline has a top-level list of `sections`. Each section has a `title` and a list of `chapters`. Each chapter groups related hunks. Sections are the coarse split (e.g. "Backend", "Frontend", "Misc"), chapters are the narrative units inside.

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

Save this as the starting storyline JSON. The existing sections, chapters, descriptions, and notes should be used as the basis — preserve what's there and only adjust for hunks that have changed in the new diff.

## Step 2: Analyze the diff

Parse the diff and list every file and hunk. For each file, count the hunks (0-indexed). Present a summary like:

```
src/main.rs: 3 hunks (0, 1, 2)
src/lib.rs: 1 hunk (0)
README.md: 1 hunk (0)
```

Read the actual source files to understand what the changes do.

## Step 3: Choose your sections

Before writing any chapters, decide on the **section structure**. Sections are how you carve a PR into top-level groups. Most PRs need only one or two sections; large PRs may need more. Use these heuristics:

- **Monorepo / multi-package PR** → one section per project or package (e.g. "Backend", "Frontend", "Mobile", "Infra"). Reviewers can read just the parts relevant to them.
- **Mass renames, formatting-only churn, generated code** → push into a **"Misc"** section so the substantive narrative stays uncluttered. Misc is the conventional dumping ground for noise: import reordering, lint fixups, codegen output, dependency bumps that touched many files.
- **Localization / translations** → a separate section ("Localization" or "i18n"). String files are bulky and benefit from being read together, away from logic changes.
- **Tests** → usually keep tests inside the same section as the code they cover (so the chapter can pair the change with its test). Only split tests into their own section if they're a large standalone addition (e.g. a brand-new test suite).
- **Documentation** → a "Docs" section if doc changes are substantial; otherwise fold doc updates into the relevant feature chapter.
- **Database migrations / schema** → either their own section ("Schema") or the first chapter of the backend section, depending on size.

If the PR is focused and small, a single section (e.g. "Main" or named after the feature) is fine. **Don't manufacture sections** — only split when it genuinely helps the reviewer.

Section ordering matters: put the most important / most read sections first. Misc-style sections last.

## Step 4: Open the HTML viewer

Before building the storyline, write an initial empty storyline JSON and generate the HTML:

```
diffstory view --open --story <path> --diff <diff-path>
```

The `--open` flag opens `/tmp/diffstory.html` in the browser. On subsequent updates, re-run without `--open` (the file is overwritten in place) and ask the user whether they want to **open in browser** (use `--open`) or just **refresh** their existing tab. Use AskUserQuestion for this.

## Step 5: Build the storyline interactively

Propose your section structure and an initial chapter list within each section. Each chapter should group related hunks that tell a coherent part of the story. Present your proposal and ask the user to confirm or adjust.

For each chapter, draft:
- **title**: concise name for this group of changes
- **description**: markdown explanation of what this chapter covers and why
- **hunks**: which file + hunk_index pairs belong here
- **notes**: optional inline annotations for specific hunks that need extra context

After each round of edits, re-run `diffstory view --story <path> --diff <diff-path>` to regenerate the HTML, and ask the user if they want to open it in the browser or just refresh their existing tab.

## Step 6: Validate coverage

Write the updated storyline JSON and run:

```
diffstory validate --story <path> --diff <diff-path>
```

Every hunk must be assigned to exactly one chapter (in any section). If coverage is not 100%, identify the missing hunks and ask the user where they belong — usually they go into a Misc section.

Iterate until validation shows 100% coverage, re-running `diffstory view` after each change.

## Step 7: Embed in PR

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
  "sections": [
    {
      "title": "Backend",
      "description": "Optional section-level note (markdown)",
      "chapters": [
        {
          "title": "Chapter title",
          "description": "What and why (markdown)",
          "hunks": [
            { "file": "api/handler.rs", "hunk_index": 0, "note": "Optional annotation" }
          ]
        }
      ]
    },
    {
      "title": "Frontend",
      "chapters": [
        { "title": "New components", "hunks": [{ "file": "web/src/App.tsx", "hunk_index": 0 }] }
      ]
    },
    {
      "title": "Misc",
      "chapters": [
        { "title": "Routine Updates", "hunks": [{ "file": "README.md", "hunk_index": 0 }] }
      ]
    }
  ]
}
```

## Guidelines

- Group hunks by **logical concern**, not by file. A single file's hunks may span multiple chapters.
- Write descriptions that explain **why**, not just what. Reviewers can see the code — they need the narrative.
- Keep chapter titles short (3-6 words). Section titles are also short — usually one word.
- **Avoid per-hunk notes.** A chapter's description should be sufficient to explain all its hunks. Only add a `note` to a hunk when it would be genuinely confusing without one (e.g. a non-obvious side effect, a subtle ordering dependency). Most chapters should have zero notes.
- Order chapters within a section to tell a story: setup before usage, core logic before edge cases.
- Prefer **fewer, larger sections** over many small ones. If you'd have a section with a single chapter and the title doesn't carry meaning, fold it into a neighboring section instead.

## PR Comments in the Viewer

When viewing a PR URL, the HTML viewer automatically shows:
- **Inline review comments** at the exact diff lines they reference, with threaded replies
- A **right panel** with all comments grouped by file, scroll-synced to the diff viewport
- **Issue comments** in a "Discussion" section in the right panel
- **Resolved threads** in a collapsible section (auto-detected via GitHub's resolved state)
- **Bot comments** filtered into their own collapsible section
- **Outdated comments** in a collapsible section for comments that no longer map to current lines
- A **comments toggle** in the right panel header to show/hide all comments and the panel

Users can click diff line numbers to draft new comments — the form appears in the right panel's draft area. The **Export** button in the right panel copies a batch shell script with all `gh api` commands to the clipboard — run it to post all comments at once. Drafts are auto-saved to localStorage.
