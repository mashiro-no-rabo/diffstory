---
name: diffstory
description: Generate a narrative storyline from a specified diff, or view an existing diffstory from a PR URL. Use when you want to create or view a guided code review story.
allowed-tools: Bash, Read, Grep, Glob, Write, AskUserQuestion
---

## Storage

If the target is a single jj change, use `.diffstory/<change id>/`.

If unspecified, generate a unique session ID (e.g. using `mktemp -d /tmp/diffstory-XXXXXX`) and save all files under that directory — e.g. `$SESSION/diff.patch` and `$SESSION/story.json`. This allows multiple sessions to work on different diffs in parallel without overwriting each other.

### Share diffstory in PR description

Diffstories for a PR should be put in a hidden block at the end of a PR description on GitHub:

```
<details><summary>diffstory</summary><!--diffstory:...base64 encoded...--></details>
```

**CRITICAL**: Never load the encoded base64 string into context — it's opaque data. Pipe everything through files:

Use `diffstory encode --story <path> --wrap` to generate encoded base64 representation of the diffstory.

If a `<details>` block with `<!--diffstory:` exists, replace **only** that block with the new encoded output. Do NOT modify any other lines.

## Viewing an existing diffstory

If the user provides a **GitHub PR URL** and wants to view (not generate) a diffstory, simply run:

```
diffstory view --open <PR_URL>
```

This fetches the PR diff and embedded storyline, generates the HTML viewer, and opens it. Review comments and issue comments from the PR are automatically fetched and displayed inline. Do NOT proceed with the generation steps below.

---

## Generating a new diffstory

You are building a **diffstory** — a narrative that organizes a PR's diff hunks into **sections**, gathered under one or more **groups**, so reviewers can follow the author's intent.

A storyline has a top-level list of `groups`. Each group has a `title` and a list of `sections`. Each section groups related hunks. Groups are the coarse split (e.g. "Backend", "Frontend", "Misc"), sections are the narrative units inside.

## Step 1: Analyze diff

Check if the current change has multiple parents with `jj log -r @-`, ask the user which parent to diff against before proceeding.

Generally should compare the working copy change against the default branch.

Read the actual source files to understand what the changes do.

## Step 2: Group logical changes

Before writing any sections, decide on the **group structure**. Groups are how you carve a PR into top-level buckets. Most PRs need only one or two groups; large PRs may need more. Use these heuristics:

- **Monorepo / multi-package PR** → one group per project or package (e.g. "Backend", "Frontend", "Mobile", "Infra"). Reviewers can read just the parts relevant to them.
- **Mass renames, formatting-only churn, generated code** → push into a **"Misc"** group so the substantive narrative stays uncluttered. Misc is the conventional dumping ground for noise: import reordering, lint fixups, codegen output, dependency bumps that touched many files.
- **Localization / translations** → a separate group ("Localization" or "i18n"). String files are bulky and benefit from being read together, away from logic changes.
- **Tests** → keep small tests in the same group as the code they cover. Split tests into their own group if they're a large standalone addition (e.g. a brand-new test suite).
- **Documentation** → a "Docs" group if doc changes are substantial; otherwise fold doc updates into the relevant feature section.
- **Database migrations / schema** → either their own group ("Schema") or the first section of the backend group, depending on size.

If the PR is focused and small, a single group (e.g. "Main" or named after the feature) is fine. **Don't manufacture groups** — only split when it genuinely helps the reviewer.

Group ordering matters: put the most important / most read groups first. Misc-style groups last.

Other guidelines:

- Group hunks by **logical concern**, not by file. A single file's hunks may span multiple sections.
- Write descriptions that explain **why**, not just what. Reviewers can see the code — they need the narrative at a higher level.
- Write descriptions using bullet points, and short, concise lines. Don't slop long paragraphs.
- Keep section titles short (3-6 words). Group titles are also short — usually one word.
- **Avoid per-hunk notes.** A section's description should be sufficient to explain all its hunks. Only add a `note` to a hunk when it would be genuinely confusing without one (e.g. a non-obvious side effect, a subtle ordering dependency). Most sections should have zero notes.
- Order sections within a group to tell a story: setup before usage, core logic before edge cases.
- Prefer **fewer, larger groups** over many small ones. If you'd have a group with a single section and the title doesn't carry meaning, fold it into a neighboring group instead.

## Step 3: Build the story interactively

Gather the diff (`jj diff`) into a file and generate an initial story with your best effort using the story JSON format below, then run:

```
diffstory view --open --story <path> --diff <diff-path>
```

The `--open` flag opens `/tmp/diffstory.html` in the browser.

Always iterate on the story with the user, ask if the user wants to make any changes in grouping, re-run the same command without `--open` in future iterations.

Use `diffstory validate --story <path> --diff <diff-path>` to check for coverage, aim for 100%.

## Story JSON format

```json
{
  "description": "Overall reading guide (markdown)",
  "groups": [
    {
      "title": "Backend",
      "description": "Optional group-level note (markdown)",
      "sections": [
        {
          "title": "Section title",
          "description": "What and why (markdown)",
          "hunks": [
            {
              "file": "api/handler.rs",
              "hunk_index": 0,
              "note": "Optional annotation"
            }
          ]
        }
      ]
    },
    {
      "title": "Frontend",
      "sections": [
        {
          "title": "New components",
          "hunks": [{ "file": "web/src/App.tsx", "hunk_index": 0 }]
        }
      ]
    },
    {
      "title": "Misc",
      "sections": [
        {
          "title": "Routine Updates",
          "hunks": [{ "file": "README.md", "hunk_index": 0 }]
        }
      ]
    }
  ]
}
```
