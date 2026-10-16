---
name: rust-tests-pass
description: Ensure all dcmtagger tests pass (`cargo test`) and that new or changed logic is covered by unit tests. Use after every code change and before finishing a task.
---

# All Tests Pass

CI runs `cargo test --release`. The whole suite must be green.

## Steps

1. Run the full suite:
   ```sh
   cargo test
   ```
   While iterating, filter by module or name, e.g. `cargo test tree_widget::` or `cargo test test_level`, then always finish with the full run.
2. The output must end with `test result: ok.` and `0 failed`.
3. If a test fails, determine whether your change or the test is wrong. Fix the code at the root cause; only adjust a test if its expectation is actually outdated, and tell the user why.

## Writing Tests

- Tests live inline in `#[cfg(test)] mod tests { use super::*; ... }` at the bottom of the module (see `src/tree_widget.rs`).
- Add tests for new logic that can run headless: tree navigation, DICOM value formatting, VR validation in `src/tag_edit.rs`, `AppCmd` execute/undo.
- Do not start the TUI in tests. Test pure functions and state transitions instead of rendering to a real terminal; for widgets, render into a `ratatui::buffer::Buffer` if needed.
- Real DICOM files are in `testdata/` (`test.dcm`, `test2.dcm`, Git LFS). Reference them relative to the crate root, e.g. `Path::new("testdata/test.dcm")`. If a file is an LFS pointer (not fetched), the test should fail with a clear message rather than pass silently.
- Never write into `testdata/`. Copy files to a temporary directory (`std::env::temp_dir()`) when testing save behavior.
- Optional/large data tests (like `spine-phantom/` in `src/dicom.rs`) must skip themselves when the data is missing.

## Rules

- Never delete, `#[ignore]`, or weaken tests just to get a green run.
- Tests must be deterministic and must not depend on the terminal, network, or local machine state.
