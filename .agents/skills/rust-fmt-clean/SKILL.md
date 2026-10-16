---
name: rust-fmt-clean
description: Ensure the dcmtagger Rust code has no rustfmt differences. Use after every Rust code change and before finishing a task; CI fails on `cargo fmt --all -- --check`.
---

# No Format Warnings

The CI step `cargo fmt --all -- --check` must pass. The project config lives in `rustfmt.toml` (`max_width = 140`, `fn_call_width = 85`).

## Steps

1. After editing Rust files, run:
   ```sh
   cargo fmt --all
   ```
2. Verify:
   ```sh
   cargo fmt --all -- --check
   ```
   No output and exit code 0 means clean.
3. If `cargo fmt` reformats code you did not touch, keep those changes minimal and mention them to the user. Do not mix large unrelated reformatting into a feature change.

## Rules

- Never change `rustfmt.toml` to make a check pass unless the user asks for it.
- Do not use `#[rustfmt::skip]` unless there is a strong reason (e.g. a hand-aligned table); if used, add a short comment explaining why.
- Long lines up to 140 characters are intentional in this project; do not wrap them manually.
