# Cycle 002 — Style conformance: REVIEW

Reviewer: child agent-2. Read-only review; the only file created is this one.
Repo root: `/Users/oliverwilliams/Documents/thoth`.

## Tree state

Not a git repository in this sandbox (`git status` → "fatal: not a git
repository"), so scope is checked by file presence and content, not by
`git diff`. The hashed path set below is unchanged by this review (this
file lives under `specs/`, outside the set).

Before inspection:

```
582c022a2c9756fcb3f6b3fb886f95ff7e91966a  benches/fill_write.rs
a496e984e40d5ae3884801a0e37da3a0c17956e6  Cargo.lock
1e97ae662767b8693fd65b85c43175e6081bf5c2  Cargo.toml
bb8c70e0fd30e66278c640a7957c9a89bec14155  check.sh
8fe5498f7b7c3a49e0b7de9f00ee3861d5430045  examples/bench.rs
00f1cbb588dc8576fb802aa8fc8d1f4fe9c86717  src/contract.rs
f8fbf5c854c19bde1d68f01befdb1be7f6e562e7  src/emit.rs
2b46cec7eaa48a49a60e282c49e29b3915c21291  src/lib.rs
b0abaaad7e3ff71d5150901ac2de5b19cb98560a  src/xlsx.rs
a39f64ce5a0ca535266727991aadbda76eb211d5  src/zip.rs
56379ce02484e4259a16f46141907b4ca11910c7  tests/common/mod.rs
9dc16dc3e765486d0d0c773ef53c22e20c01171c  tests/roundtrip.rs
```

After inspection: identical (same 12 lines, same hashes).

## Requirement verdicts

1. **Remove the callerless dependency — PASS.** `Cargo.toml:15-16`
   `[dev-dependencies]` holds only `criterion = "0.5"`, no calamine.
   `grep -c calamine Cargo.lock` → 0; no calamine match in any scoped
   file (`Cargo.toml`, `Cargo.lock`, `src/emit.rs`, `src/zip.rs`,
   `src/xlsx.rs`, `tests/common/mod.rs`, `check.sh`, `STYLE.md`).
   `criterion` caller confirmed:
   `benches/fill_write.rs:15` (`use criterion::{...}`),
   `benches/fill_write.rs:98-99` (`criterion_group!`/`criterion_main!`).
2. **Remove both glob imports — PASS with one form note.**
   `src/emit.rs:88`: `use super::{NULL_I64, col_letters, f64_cell,
   i64_cell};` — exactly the four names the test body uses
   (`src/emit.rs:91-106`). `src/zip.rs:128`: `use super::crc32;` — the
   one name its test body uses (`src/zip.rs:131-133`).
   `grep -rn 'super::\*'` over `src tests benches examples` returns
   nothing. Note: the zip import is the unbraced single-item form
   rather than the braced `use super::{…}` spelling in the SPEC; it is
   semantically itemized (glob gone, rule satisfied), recorded here as
   an observation, not a failure. The emit import resolves `NULL_I64`
   through the parent's `use crate::contract::NULL_I64`
   (`src/emit.rs:5`); the green gate below proves it compiles.
3. **Refuse the wildcard — PASS.** `src/xlsx.rs:113-122`: `source()`
   matches `Xlsx`, `Io`, `Rename` with `Some`, and spells
   `WriteError::Exists(_) | WriteError::TempExists(_) => None`
   (`src/xlsx.rs:119`) — no `_` arm. Same semantics (the two
   data-less variants still map to `None`); error strings untouched
   (`src/xlsx.rs:91-111` unchanged in shape).
4. **Annotate unchecked bindings — PASS.** All seven named bindings
   carry `: usize`, and RHS-naming bindings stay bare:
   `src/xlsx.rs:191` (`cap`), `src/xlsx.rs:195` (`row_cap`),
   `src/zip.rs:61` (`total`), `tests/common/mod.rs:243` (`at`),
   `tests/common/mod.rs:253` (`name_at`), `:254` (`data_at`), `:255`
   (`end`). Bare where the SPEC requires bare: `src/zip.rs:66`
   (`Vec::with_capacity`), `src/zip.rs:48` (`crc32(&data)` return),
   `src/xlsx.rs:186` (`.collect()` names the type via the
   `Vec<String>` annotation on the same line). No strict `.unwrap()` /
   `.expect(` / `.into()` in `src/` (the one `unwrap` substring hit is
   `unwrap_or` at `src/xlsx.rs:252`, which is not the refused method).
5. **Pin the gate — PASS.** `check.sh:4` `cargo fmt --check` (no flag,
   as required); `check.sh:5` `cargo test --locked`,
   `check.sh:6` `cargo clippy --locked`, `check.sh:7`
   `cargo doc --locked`.
6. **Declare endianness — PASS.** `src/zip.rs:117`: `// Multi-byte
   fields are little-endian per the ZIP format.` One line, on the
   writer helpers. The CRC-32 IEEE doc line is kept
   (`src/zip.rs:8-9`).
7. **Close the register — PASS.** `STYLE.md:151-152`: row 1 closed
   (glob removal, both files named), row 2 ruled refused (`Exists`,
   `TempExists` named). Rows 3-4 present and ruled. Whether "nothing
   else in `STYLE.md` changes" holds is unverifiable without a git
   baseline; the current body is fully consistent with the register
   requirement and no other rule text looks cycle-002-related.

## Acceptance verdicts

- **`sh check.sh` green — PASS.** Ran with
  `CARGO_HOME=/tmp/cargo-home` exported: `cargo fmt --check`, `cargo
  test --locked`, `cargo clippy --locked`, `cargo doc --locked` all
  pass; output ends `all gates green` (3 unit + 1 round-trip test
  suites green, 0 failures).
- **Glob/calamine grep — PASS with scoping note.**
  `super::\*`: zero matches in `src tests benches examples`.
  `calamine`: zero matches in `Cargo.lock` and in every scoped file.
  The only remaining mentions are pre-existing, out-of-scope text:
  `specs/002-style-conformance/SPEC.md` (the task text itself) and
  `TESTING.md:44,88` (the specified-but-not-yet-installed reader
  oracle). The recorded removal decision belongs in
  `specs/002-style-conformance/DECISION.md`, which is correctly absent
  (see next item), so those two pre-existing mentions are not a
  violation of this cycle.
- **Scoped-files-only diff — CONDITIONAL PASS (evidence gap).** No git
  history exists in this sandbox, so a `git`-style diff review is
  impossible. Positive evidence: the producer's reported file list
  (`Cargo.toml`, `Cargo.lock`, `src/emit.rs`, `src/zip.rs`,
  `src/xlsx.rs`, `tests/common/mod.rs`, `check.sh`, `STYLE.md`) is
  exactly the SPEC scope; out-of-scope files inspected read clean
  (`src/contract.rs`, `src/lib.rs`, `benches/fill_write.rs`,
  `examples/bench.rs`, `tests/roundtrip.rs` show no style-batch
  residue); `specs/002-style-conformance/` contains only `SPEC.md`
  plus this review. Line-level review above shows renames,
  annotations, flags, and one comment — no logic changes.
- **Null behavior / byte diff — PASS.** Tests green (see gate run);
  `tests/roundtrip.rs:16-41` still round-trips every example through
  disk against unmodified `expected` vectors plus the per-example
  verbatim sheet-embed check; `tests/common/mod.rs` expectation
  constructors (`ints_floats` through `infinite`, `all`) are intact.
  No error-message, taxonomy, capacity-math, or bench-scale edits
  found in the inspected files.
- **`DECISION.md` left for the applicator — PASS.**
  `specs/002-style-conformance/` holds only `SPEC.md` (and now this
  `REVIEW.md`); no `DECISION.md` was created by the producer.

## Findings

1. (Observation, non-blocking) `src/zip.rs:128` uses `use
   super::crc32;` instead of the braced `use super::{crc32};` form the
   SPEC names. Semantically itemized; suggest normalizing to braces
   only if a future touch is already editing that line.
2. (Evidence gap, environmental) No git baseline → the "diff touches
   only scoped files" and "nothing else in `STYLE.md` changes" clauses
   are verified by content inspection, not by diff. Recommend the
   applicator confirm with `git diff --stat` on a machine with history.
3. No source edits were made by this reviewer; `find` hashes before
   and after are identical.

## Overall verdict: PASS (conditional on finding 2's applicator-side diff check)
