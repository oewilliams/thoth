# Cycle 002 — Style conformance batch: SPEC

Fix the six review findings against `STYLE.md`. No behavior change, no
byte change, no performance work. The round-trip tests plus verbatim
checks prove the null diff.

## Scope

`Cargo.toml`, `Cargo.lock`, `src/emit.rs`, `src/zip.rs`, `src/xlsx.rs`,
`tests/common/mod.rs`, `check.sh`, `STYLE.md` (register only). Anything
else sends the cycle back.

## Requirements

1. **Remove the callerless dependency.** Delete `calamine = "0.36"`
   from `[dev-dependencies]` in `Cargo.toml` and regenerate
   `Cargo.lock` so it no longer mentions calamine (run the gate; cargo
   rewrites the lock). `criterion` stays: its caller is
   `benches/fill_write.rs`.
2. **Remove both glob imports.** `src/emit.rs` and `src/zip.rs` test
   modules use `use super::*`; replace with itemized `use super::{…}`
   naming exactly what each test body uses. No test expectation
   changes.
3. **Refuse the wildcard.** `WriteError::source` in `src/xlsx.rs`
   ends in `_ => None`; spell the remaining variants explicitly so the
   next added variant breaks compilation. Same semantics, no message
   changes.
4. **Annotate unchecked bindings.** Add type annotations where the
   right-hand side does not name the type, strictly in library code
   (`cap`, `row_cap` in `src/xlsx.rs`; `total` in `src/zip.rs`) and
   for the index arithmetic in the test reader (`at`, `name_at`,
   `data_at`, `end` in `tests/common/mod.rs`). Bindings whose RHS
   names the type (constructors, `T::from`, casts with `as`, function
   returns read at the call's own line) stay bare: repetition is noise.
5. **Pin the gate.** Add `--locked` to the `cargo test`, `cargo
   clippy`, and `cargo doc` invocations in `check.sh` (not to `cargo
   fmt`, which takes none), so identical inputs stay identical outputs
   per the reproducible-gate rule.
6. **Declare endianness.** One comment line on the zip writer stating
   multi-byte fields are little-endian per the format (CRC-32 IEEE is
   already documented; keep that line).
7. **Close the register.** Update `STYLE.md` register rows 1 and 2 to
   record the glob removal and the wildcard ruling. Nothing else in
   `STYLE.md` changes.

## Non-goals (do not touch)

Emitted bytes, error messages or taxonomy, CRC structure, staging,
formatting, capacity math, benches and their scales, test
expectations, new dependencies, any tuning.

## Acceptance

- `sh check.sh` green, including the new `--locked` flags (note: in
  this sandbox run all cargo commands with
  `CARGO_HOME=/tmp/cargo-home` exported; that workaround is
  environment, not code).
- `grep -rn "super::\*"` over the repo returns nothing; `calamine`
  appears nowhere except the recorded decision below.
- Diff touches only the scoped files, and `git`-style review of it
  shows renames, annotations, one flag, and one comment — no logic.
- Record in `specs/002-style-conformance/DECISION.md` (applicator's
  file, producer leaves it for them): calamine removed per review
  finding 1, to be re-added by the reader-wiring cycle at one commit
  of cost.
