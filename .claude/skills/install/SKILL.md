---
name: install
description: Install diffstory binary and register the storyline generator skill
---

Install diffstory by running these steps in order:

1. Check that `cargo` is available:
   ```
   cargo --version
   ```
   If this fails, tell the user they need to install Rust first via https://rustup.rs/ and stop.

2. Check that `gh` (GitHub CLI) is available:
   ```
   gh --version
   ```
   If this fails, tell the user they need to install it from https://cli.github.com/ and stop.

3. Build and install the binary:
   ```
   cargo install --path .
   ```
   If this fails, stop and show the error.

4. Symlink the storyline generator skill so it's available globally as `/diffstory`:
   ```
   ln -sf "$(pwd)/skills/diffstory" "$HOME/.claude/skills/diffstory"
   ```
   If the symlink target already exists, the `-f` flag will replace it.

Report the result of each step.
