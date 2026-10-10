# Cycle 001 — Floor review: REVIEW

Reviewer verdict: **PASS**. All six SPEC requirements and all four
acceptance items hold on the inspected tree. One non-blocking
observation is recorded for a future cycle.

## Scope record (against baseline `4e22bef`)

`git status --short` at review time:

```text
M src/xlsx.rs
 M src/zip.rs
```

(`M` staged + unstaged for `src/xlsx.rs`, unstaged for `src/zip.rs`;
no other entries — no untracked files.)

`git diff --stat HEAD` at review time:

```text
 src/xlsx.rs |  59 +++++++++++++++++++++++++++------
 src/zip.rs  | 107 +++++++++++++++++++++++++++++++++++++++++++++++++++++++++++-
 2 files changed, 155 insertions(+), 11 deletions(-)
```

`git diff --stat HEAD -- tests/ src/emit.rs src/contract.rs Cargo.toml
Cargo.lock benches/ examples/` is empty: no test expectations, no
oracle, no bench/scale, no dependency changes. The only test changes
are additive `#[cfg(test)]` unit modules inside the two scoped files,
which the SPEC allows.

## Requirement verdicts

1. **4GB boundary guard (PASS).** `stored_archive_fits`
   ([src/zip.rs](/Users/oliverwilliams/Documents/thoth/src/zip.rs:53))
   checks every SPEC-enumerated cast category explicitly, and each
   check maps to its cast in `ZipWriter::finish`
   ([src/zip.rs](/Users/oliverwilliams/Documents/thoth/src/zip.rs:114)):
   per-part data length (`u32`, finish lines 130–131, 147–148) via
   `data_len > U32_MAX` (line 64); local header offsets (`u32`,
   lines 122, 137, 155) via the running `offset > U32_MAX` check
   (lines 67–69) and the final `offset <= U32_MAX` (line 73);
   central directory offset and length (`u32`, lines 158, 164–165)
   via the final `offset <= U32_MAX && central_len <= U32_MAX`
   (line 73); entry counts (`u16`, lines 162–163) via
   `entries.len() as u64 > U16_MAX` (lines 56–58); name lengths
   (`u16`, lines 132, 149) via `name_len > U16_MAX` (line 64).

2. **Check placement (PASS, with note).** The guard runs in
   `build_xlsx` ([src/xlsx.rs](/Users/oliverwilliams/Documents/thoth/src/xlsx.rs:259))
   before `ZipWriter::new` / any assembly byte (lines 264–268), so no
   narrowing cast ever executes on a refusing input. Note: it runs
   after `sheet_xml` formatting (line 238), because the sheet's
   length — the guard's input — exists only once formatted. This is
   sound, not a bypass: the formatting path was verified to contain
   no narrowing `u32`/`u16` casts (`grep " as "` over `src/emit.rs`,
   `src/contract.rs`, `src/xlsx.rs` finds only `u8`/`char` widenings
   in `col_letters`), so the single pre-assembly check covers every
   cast on the path, as the code comment (lines 250–254) states.

3. **New error variant (PASS).** `XlsxError::TooLarge { bytes: usize }`
   ([src/xlsx.rs](/Users/oliverwilliams/Documents/thoth/src/xlsx.rs:37)),
   additive variant only; `Display` states the ceiling in existing
   lowercase style, `"archive too large {bytes}: zip stops at {}"`
   with `u32::MAX` (lines 64–66); the blanket `impl Error for
   XlsxError` (line 84) covers it. Unit test asserts the ceiling
   appears (lines 300–304).

4. **Pure predicate, tested at the boundary (PASS).**
   `stored_archive_len` (lines 28–37) and `stored_archive_fits`
   (lines 53–74) are pure functions of `&[(usize, usize)]`, `u64`
   arithmetic, no allocation. `archive_fits_flips_at_the_u32_ceiling`
   ([src/zip.rs](/Users/oliverwilliams/Documents/thoth/src/zip.rs:195))
   decides sizes only — no bytes allocated — at data lengths
   `u32::MAX - 1`, `u32::MAX`, `u32::MAX + 1` (lines 204–206, all
   refuse) plus the exact flip point `MAX - 30` fits / `MAX - 29`
   refuses (lines 202–203). Offset accumulation across entries, `u16`
   name bound, `u16` count bound, and central-length overflow under
   fitting `u16` fields are each tested (lines 209–238). The `u16`
   count tests allocate only small `(0, 0)` tuples (~1 MB at most).

5. **Byte oracle lock (PASS).** `tests/roundtrip.rs` (per-example
   disk round-trip plus the `zip does not embed sheet verbatim`
   check, line 36) and `tests/common/` are byte-identical to HEAD
   (empty diff, see scope record) and green — see gate evidence
   below. Zero byte changes for in-range inputs by construction:
   the happy path adds only a predicate evaluation over six size
   pairs before the unchanged assembly sequence.

6. **Docs (PASS).** `///` comments on all three new items
   (`stored_archive_len` lines 22–27, `stored_archive_fits` lines
   39–52, `TooLarge` lines 37–39). The `cargo doc` gate passed with
   `-D`-equivalent denial of warnings via `check.sh`.

## Acceptance verdicts

- **`sh check.sh` green (PASS).** Ran with
  `CARGO_HOME=/tmp/cargo-home` exported (sandbox workaround,
  environment only): `cargo fmt --check`, `cargo test` (10 unit +
  1 doc-adjacent passing suites, 0 failed), `cargo clippy
  --all-targets -- -D warnings`, `cargo doc --no-deps` — final line
  `all gates green`.
- **Boundary unit tests pass in both profiles (PASS).**
  `cargo test --release --locked` repeats the same 10 + 1 green,
  0 failed.
- **Floor bench within noise (PASS).** `cargo bench --bench
  fill_write floor`: `tall_null_20000` ~7.57 ms vs 7.6 ms baseline,
  `tall_null_200000` ~78.7 ms vs 80.5 ms baseline (`TESTING.md`
  lines 66–68); criterion reports "No change in performance
  detected" for both.
- **Diff scope (PASS).** Only `src/zip.rs`, `src/xlsx.rs`, additive
  unit tests therein. Nothing else.

## Findings

- **Style.** New code follows the register: itemized test imports
  (`src/zip.rs:182`, `src/xlsx.rs:298` — no `use super::*`),
  no `unwrap`/`expect` in `src/` (the single `unwrap_or` at
  `src/xlsx.rs:261` is `Option::unwrap_or`, not in the refused
  family), conversions target-spelled (`usize::try_from`,
  `u64` annotations), `overflow-checks = true` in release
  (`Cargo.toml:10`) preserved.
- **Observation (non-blocking, future cycle).** `ZipWriter::finish`
  derives the end-record central length as `out.len() as u32 -
  central_start` (`src/zip.rs:158`). The guard bounds the local
  section and the central section each against `u32::MAX` but not
  their sum, so a ~4 GB input whose sections individually fit (e.g.
  the fits-true `[(u32::MAX - 30, 0)]` size case, total
  `u32::MAX + 68`) would truncate that one cast. No `u32` field
  holds the archive total, so this is outside the SPEC's enumerated
  categories and unreachable without gigabyte allocation — it does
  not fail any requirement — but a later cycle touching `finish`
  should compute the end-record length from the tracked section
  lengths instead of re-narrowing `out.len()`.
