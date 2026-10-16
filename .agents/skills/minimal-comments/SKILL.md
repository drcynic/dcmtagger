---
name: minimal-comments
description: Keep code comments in dcmtagger minimal and in English - only where the code cannot explain itself (hacks, workarounds, non-obvious bug fixes, external constraints). Use whenever writing or reviewing Rust code.
---

# Minimal Comments

Code should explain itself through names, types, and structure. Comments are the exception, not the rule.

## Write a comment only when

- A **hack or workaround** is needed (e.g. a terminal or library quirk) – explain what and why, ideally with a link.
- A **bug fix** is not obvious from the code – explain what failed before, so nobody "simplifies" it back.
- An **external constraint** applies – DICOM standard rules (cite the PS3 part/section), Kitty graphics protocol parameters, terminal behavior.
- A **non-obvious invariant or performance decision** exists that the type system does not express.
- A `#[allow(...)]` attribute is used – one short line why.

## Do not write comments that

- Restate the code (`// increment counter`, `// render the block`, `// create text area`).
- Label obvious sections (`// Build layout`, `// Optional error row`).
- Describe what a well-named function or variable already says.
- Contain commented-out code – delete it; Git keeps the history.
- Track history or authorship (`// changed by ...`, `// new version`).

## Instead of a comment

- Rename the variable or function so the intent is clear.
- Extract a well-named helper function or constant (e.g. `const CELL_WIDTH_OVER_HEIGHT`).
- Use a type or enum variant that encodes the meaning.

## Style

- Always English, short, one line where possible.
- Explain **why**, not **what**.
- Doc comments (`///`) only for items whose usage is not obvious from the signature.
- Do not remove existing meaningful comments; when touching code, remove only clearly redundant comments in the lines you change.
