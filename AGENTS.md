# dcmtagger – Project Overview for Agents

Terminal UI (TUI) for viewing and editing DICOM tags, written in Rust (edition 2024).
Author: Daniel Szymanski. Status: proof of concept, active development on `main`.

## Language

- All files (code, comments, docs, commit messages, skills) are written in **English**.
- Chat with the user may be in German.

## Quality Rules (mandatory)

Before finishing any task, apply these skills (in `.agents/skills/`):

| Skill               | Rule                                                                                           |
| ------------------- | ---------------------------------------------------------------------------------------------- |
| `rust-fmt-clean`    | `cargo fmt --all -- --check` reports no differences                                            |
| `rust-clippy-clean` | `cargo clippy --all-targets --all-features -- -D warnings` reports no warnings                 |
| `rust-tests-pass`   | `cargo test` is fully green                                                                    |
| `minimal-comments`  | Comments only where the code cannot explain itself (hacks, workarounds, non-obvious bug fixes) |

CI (`.github/workflows/ci.yml`) checks fmt, clippy (`--release`, `-D warnings`), tests (`--release`) and a release build on every push to `main`.

## Build & Run

```sh
cargo build
cargo run -- <file.dcm | directory> [-d|--debug] [-s|--skip-pixel-data]
cargo test
```

- The app is an interactive TUI (ratatui/crossterm) and cannot meaningfully run headless. Do not launch it as an agent; verify behavior through tests.
- Test data: `testdata/test.dcm`, `testdata/test2.dcm` (Git LFS, see `.gitattributes`). Not referenced by tests yet. `local_testdata/` is gitignored.
- The timing test in `dicom.rs` uses `spine-phantom/` and skips itself if the directory is missing.

## Formatting

`rustfmt.toml`: `max_width = 140`, `fn_call_width = 85`. Long lines are intentional.

## Dependencies (Cargo.toml)

`ratatui 0.29`, `crossterm 0.29`, `tui-textarea 0.7`, `dicom-* 0.8` (core, object, encoding, dictionary-std, transfer-syntax-registry), `slotmap` (tree node IDs), `clap` (CLI), `anyhow`, `base64` (Kitty graphics protocol).

## Architecture

```
src/
├── main.rs          Entry point: parse CLI, ratatui::init, App::run, restore
├── app.rs           App state, modes, key handling, rendering, undo/redo stacks
├── app_cmd.rs       Command pattern: trait AppCmd, TagEditCmd, MacroCmd (not used yet)
├── dicom.rs         DicomData: loading files/directories, tree building, value formatting
├── tree_widget.rs   Generic tree (SlotMap) + navigation + TreeWidgetRenderer; most unit tests live here
├── tag_edit.rs      Edit popup: VR-dependent input/validation, produces a new DataElement
├── help.rs          Scrollable help overlay (text duplicates the README key bindings)
└── kitty_image.rs   Image display via Kitty graphics protocol (escape sequences on stdout)
```

### Core Types

- `App<'a>` (`app.rs`): owns `DicomData`, `TreeWidget`, `Mode`, `SortOrder`, `modified_files: BTreeSet<String>`, `undo_stack`/`redo_stack: Vec<Box<dyn AppCmd + Send + Sync>>`, `handler_text` (status line), `KittyImage`.
- `Mode`: `Browse` | `Help(HelpOverlay)` | `Search` | `Edit(Box<TagEdit>)`.
- `SortOrder`: `Filename` (key `1`) | `Tag` (`2`) | `TagWithDiffs` (`3`, only tags with differing values).
- `DicomData` (`dicom.rs`): `root_path`, `datasets_by_filename: BTreeMap<String, FileDicomObject<InMemDicomObject>>`, precomputed `num_values_and_max_length_by_tag`.
- `TagElement` = `DataElement<InMemDicomObject, Vec<u8>>`.
- `TagSource { tag, filename }`: links a tree node to its element in a dataset.
- `TreeWidget` (`tree_widget.rs`): `nodes: SlotMap<Id, TreeNode>`, `root_id`, `selected_id`, `visible_start_id`, `open_nodes: HashSet<Id>`. `TreeNode { text, children, parent_id, source }`.

### Data Flow

1. `DicomData::new(path, skip_pixel_data)` reads a single file or all files of a directory (non-recursive, sorted).
2. `tree_sorted_by_filename()` / `tree_sorted_by_tag(min_diff)` each build a **new** `TreeWidget` (node texts are preformatted strings). Group nodes = tag group (`{:04x}`); sequences are inserted recursively as child nodes.
3. Edit: `i` → `switch_to_edit_mode` (only in `SortOrder::Filename`, only non-binary VRs, no `SQ`) → `TagEdit` → `State::Updated(elem)` → `TagEditCmd` via `execute_cmd` (pushed to undo stack, redo stack cleared).
4. `TagEditCmd::apply` writes the element into the dataset (`put_element`), updates the node text and marks the file as modified.
5. `w` saves all modified files to their original path (`write_to_file`).

### Image Display (WIP)

`I` toggles `show_image`. During rendering, the selected dataset's pixel data is read as `u16`, crudely reduced to 8 bit (`v / 16`), passed as RGB to `KittyImage` and uploaded. Placed right of the tree; aspect ratio uses an assumed cell ratio of 0.5.

## Known Issues / Observations

From code analysis (not all verified):

- `app.rs` image render path: `unwrap()` on `COLUMNS`/`ROWS`/`uint16_slice()` – panics on 8-bit, compressed or multi-frame data, or with `--skip-pixel-data`.
- `shown_image_id` is never set → with `show_image` active the image is re-uploaded every frame; the message in `toggle_images` therefore always shows `false`.
- `save_modified_files` clears `modified_files` even if writing failed.
- `get_value_string`: `&value_str[..77]` can panic on a non-char boundary with multibyte UTF-8.
- `tree_sorted_by_tag`: `to_int::<i32>().unwrap()` on `NUMBER_OF_FRAMES`.
- Editing only in filename sort order; `MacroCmd` (batch edit across a series) is prepared but unused (`_new`, `#[allow(dead_code)]`).
- Help text in `help.rs` and `README.md` must be kept in sync manually (key `I` is missing in both).
- `visible_nodes()` traverses the whole visible tree on every render (`!todo` in code regarding >150k nodes).

## Conventions

- Error handling: `anyhow::Result` at boundaries (loading); otherwise report via `handler_text` instead of panicking.
- New user actions: key in `App::handle_key_event` (matching `Mode`) + method on `App` + entry in `help.rs` **and** `README.md`.
- Data-changing actions are implemented as `AppCmd` so undo/redo works.
- Let chains (`if let ... && let ...`) are already used (edition 2024).
- Unit tests inline in `#[cfg(test)] mod tests` within the respective module.
