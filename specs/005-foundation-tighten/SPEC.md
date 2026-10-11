# Cycle 005 — Foundation tighten (seal, words, locks)

## Background

Cycle 004 brought `src/zip.rs` to machine tightness with proof:
assembler citations, bench isolation, a golden byte lock, and a
cast-to-check mapping. The tail of `src/xlsx.rs` (ingress guard,
assembly, sealed write, error taxonomy, frozen statics) is close in
structure but behind in proof: the seal has no refusal test, two
error positions state ambiguous bases, one error carries no
location, the statics have no byte lock, the capacity math claims
"never reallocates" without a test, and the static copies have no
measurement. Per the correction driving this cycle, these proofs
are not hygiene: they are the cycle, and they decide the fixes.

Binding references: `DESIGN.md`, `TESTING.md`, `STYLE.md`
(proof before fix; measure, don't assume — including this spec's
own suspicions, which the proofs confirm or refute).

## Requirements (proof first; each proof drives its fix)

- **R1 — Sealed-refusal locks.** Add integration tests (new
  `tests/sealed.rs`, round-trip file untouched) proving: an
  existing destination is refused with `Exists` and left
  byte-identical; a pre-existing temp file is refused with
  `TempExists` and the destination is not created; the success
  path lands beside the target and renames into place. No
  behavior change to get green unless a test fails; then the
  minimal fix the test demands.
- **R2 — Error positions state their base.** Today `Infinite.row`
  is the zero-based data index and `TooManyRows(n)` holds data
  rows while its message cites the total-row ceiling, and neither
  base is pinned beyond a column-name substring. Fix: `Infinite`
  carries the 1-based Excel row (`i + 2`, header is row 1), the
  `Display` shape otherwise unchanged; `TooManyRows` keeps its
  payload and its message states data rows and the resulting
  sheet total (`n` data rows would make `n + 1` sheet rows
  against 1048576). Document the base on each variant; pin exact
  strings in tests (the `infinite` example refuses at row 3).
  The tests are the proof; the new words are the fix they demand.
- **R3 — Rename failures carry location.** `WriteError::Rename`
  holds only `source` while `Io` holds `path` plus `source`.
  Fix: `Rename { tmp, out, source }`, displayed as
  `rename {tmp} to {out}: {source}`, `source()` unchanged
  (`Some`). Pin both paths in a test. Additive fields on one
  variant; no other taxonomy change.
- **R4 — Missing file name states its condition.**
  `write_xlsx_file` maps a missing file name to `Exists`, which
  states the wrong condition. Fix: additive
  `WriteError::NoFileName(PathBuf)`, displayed as
  `{path}: no file name`, `source()` spells it explicitly
  (`None`) so the next added variant breaks compilation, per
  register row 2. Pin with a test.
- **R5 — Statics byte lock.** The five frozen blobs have no
  tripwire: a stray `STYLES` edit changes every archive with
  only value-level round-trips to catch it. Fix: a test pinning
  each static's `(len, crc32)` (computed with the crate's own
  `crc32`, frozen as literals), commented as tripwire — the
  round-trip suite stays the oracle, mirroring the zip golden's
  own provenance note. Any static edit breaks the test on purpose.
- **R6 — Capacity proof.** `sheet_xml` claims its buffer never
  reallocates but shares no helper and pins nothing. Fix:
  extract the sizing math into a pure helper shared by
  `sheet_xml` (the `entry_sizes` pattern: one spelling feeding
  both size and use so the two cannot diverge), and test that
  the cap bounds actual output with no realloc on
  representative shapes (header-only, wide, tall at test
  scale). No emitter changes; the shape of the helper is the
  producer's, the bound is the reviewer's.
- **R7 — Static-copy measurement decides.** The five
  `as_bytes().to_vec()` copies per build have no measurement;
  borrowing them needs `ZipWriter` API churn. Measure first: a
  timed comparison on the header-only build path, recorded in
  `TESTING.md` with machine, profile, and commit. Borrow (or
  `Cow`) only if the numbers justify the churn; otherwise keep
  the copies with a comment citing the measured absolute cost.
  Relative size ("kilobytes against megabytes") is not a
  justification and is never stated; the measurement is the
  decision, either way recorded.
- **R8 — Words and numbers recorded.** `write_xlsx_file` docs
  cite caller-owned durability (rule 6) in one line; the
  `TooLarge.bytes` saturation gets the same
  try-from-or-saturate comment the zip side carries;
  `TESTING.md` records the new locks, the R7 numbers, and the
  R2 base clarification. `STYLE.md` changes only if the
  reviewer rules something new into the register.

## Scoped files

`src/xlsx.rs`, `tests/sealed.rs` (new, additive),
`tests/roundtrip.rs` only if a seal test belongs there instead
(prefer the new file), `benches/fill_write.rs` (additive only,
only if R7 needs a criterion group), `TESTING.md`,
`STYLE.md` (register only), plus `specs/005-foundation-tighten/`
records by role.

## Acceptance

- Gate green (`cargo fmt` clean, `sh check.sh` with
  `CARGO_HOME=/tmp/cargo-home`).
- Every new lock fails if its condition is broken (reviewer
  verifies by inspection or mutation, not by trust): refusal
  tests, exact error strings, statics pins, capacity bound.
- `TESTING.md` holds the R7 numbers with machine, profile, and
  commit, plus the R2 base note; percentages recompute or the
  table does not ship.
- Reviewer verdict per item in `REVIEW.md` with a proof-to-fix
  mapping (each fix names the failing proof that demanded it);
  applicator reasoning in `DECISION.md`; changes left
  uncommitted.

## Boundaries (not this cycle)

- No emitter changes: per-cell allocations, the per-cell
  `ColumnData` match, and infinity-scan fusion belong to the
  cell cycle, which re-takes the `cells` numbers. R6 bounds the
  buffer; it does not touch what fills it.
- No deflate, no template types, no row-major changes, no new
  dependencies, no run-off, no `writev`.
- No public-shape changes beyond R2's row base, R3's rename
  fields, and R4's additive variant; anything else sends the
  cycle back.
