---
name: rust-clippy-clean
description: Ensure the dcmtagger Rust code produces zero clippy warnings, matching CI (`-D warnings`). Use after every Rust code change and before finishing a task.
---

# No Clippy Warnings

CI runs `cargo clippy --release --all-targets --all-features -- -D warnings`. Every warning is an error.

## Steps

1. Run locally (debug profile is faster and reports the same lints):
   ```sh
   cargo clippy --all-targets --all-features -- -D warnings
   ```
2. Fix every reported lint at its root cause. Re-run until the output ends with `Finished` and has no `warning:` / `error:` lines.
3. Before finishing larger changes, optionally run the exact CI command with `--release` to rule out profile-specific differences.

## Rules

- Prefer fixing the code over silencing the lint. Typical fixes: use iterators instead of index loops, `is_empty()` instead of `len() == 0`, remove needless borrows/clones, collapse nested `if let` into let chains (edition 2024).
- `#[allow(clippy::...)]` / `#[allow(dead_code)]` is only acceptable when the fix would harm readability or the code is intentionally prepared for upcoming work (e.g. `MacroCmd` in `src/app_cmd.rs`). Scope it as narrowly as possible (item level, never crate level) and add a short comment explaining why.
- Do not add `#![allow(...)]` at crate level and do not change lint levels in `Cargo.toml` without asking the user.
- Tests are included via `--all-targets`; test code must be clippy-clean too.
