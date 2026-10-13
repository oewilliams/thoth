# Cycle 017 — Decision (string and bool kinds)

Applicator, after the reviewer. Machine: Darwin 27.0.0 arm64
(`RELEASE_ARM64_T8132`), Apple M4, rustc 1.93.1, bench profile
(`overflow-checks = true`, `codegen-units = 1`), HEAD `1482e88`
plus the uncommitted 017 work. Everything left uncommitted.

## Agreement with the review

Reviewer verdict: **Pass with two required changes** (R4 FAIL
narrow/pre-existing, plus the STYLE tripwire on
`validate_template`). I agree with both required changes; no
disagreement is recorded, because each names a real gap on
inspected evidence reproduced in `REVIEW.md`:

- Required change 1: `sheet_cap` priced header names at
  `name.len()` while `header_cell` escapes them at up to 6x, so
  `len <= cap` failed for escapable names (20 quotes: len 354 >
  cap 353) even though the formula pre-dates 017. R4 claims a
  true worst case and `archive_fits` rests on it, so the bound
  must price the escape.
- Required change 2: `validate_template` at 113 lines exceeded
  STYLE §1's 100-line function tripwire with no annotation
  (HEAD's version was 54 lines).

## What changed

- **Required change 1** (`src/contract.rs:462`, in `sheet_cap`
  at `:453`): header names now price at the escaped bound,
  `letter.len() + 6 * view.name.chars().count() + 64` — 6x per
  character (the widest `&quot;`/`&apos;` spelling from the same
  five-entity table as the data cells) plus the fixed header
  tags the 64 already covered. The `sheet_cap` doc now states
  header names are priced at the same bound. `sheet_hint` is
  unchanged: it is a witnessed-length reservation, not the
  proof, and correctness never depends on it. Test:
  `sheet_cap_holds` (`src/xlsx/tests.rs:201`) extended at
  `:301` with an escapable-header sheet (100 repeated quotes,
  header-only) and fifty 20-quote columns, both through the
  existing `check` (`len <= cap`) — the cases the reviewer's
  probe failed before the fix.
- **Required change 2** (`src/contract.rs:329`, `:398`): the
  string scan is extracted verbatim from `validate_template`
  into `first_text_error(views) -> Option<XlsxError>`,
  mirroring the float scan's per-column contiguous walk with
  the row-major first rebuilt from per-column firsts (earliest
  column wins a tied row, length refusal takes precedence over
  a bad char in the same cell, per-column `break` unchanged).
  The XML 1.0 valid-set spelling moves with the check into the
  helper's doc, where the check now lives. `validate_template`
  is now 57 lines (329–385), the helper 49 lines (398–446);
  no behavior, message, or ordering changes.
- **`DESIGN.md` Scope wording** (`DESIGN.md:9`): the Scope
  bullet now reads as shipped — "Cells are primitives only:
  string, int, float, bool — all four kinds ship through the
  template contract (cycle 017, as shipped:
  `Kind::I64`/`F64`/`Str`/`Bool`, inline strings, `t="b"`
  bools, blank for absent values)." No other Scope bullet
  changed; dates still ride as strings or numbers.

Only files inside the SPEC's scoped list were edited by the
applicator: `src/contract.rs`, `src/xlsx/tests.rs`,
`DESIGN.md`, and this `DECISION.md`. No `src/emit.rs`,
`src/xlsx.rs`, or `src/macros.rs` change; no new dependencies.

## R6 — null-timing resolution

I accept neither cross-binary reading as a cost verdict, and
record the discrepancy as unresolved layout-vs-cost, with no
regression claimed and no parity claimed as proven. The
Post-017 entry records interleaved parity (null HEAD
4.35–4.72ms vs post-017 4.24–4.45ms); the reviewer's
interleaved probe reads post-017 4.20–4.36ms vs HEAD
3.67–4.02ms (~+10–14%). Both compare two different binaries,
null bytes are `cmp`-identical, and the null path spells no
new cell kind (the string scan skips non-`Str` columns;
`bool`/`str` emitters are never called for `i64`-null cells),
while this codebase has already documented layout drift of
this size from unrelated spelling changes (the `#[inline]`
episode in Post-017 itself, and Post-015). A cross-binary
probe therefore cannot separate library cost from layout, as
the reviewer states. My changes cannot add null-path cost
either: the header pricing is once-per-column cold sizing
arithmetic and the extraction is a boundary-scan call, both
off the emission loop. No timing claim for the null recipe
rests on either probe; the load-bearing facts are byte
identity and the new-recipe reads, which the reviewer
reproduced exactly. The Criterion phase pairs for
`tall_strings`/`tall_bools` were not re-run by the reviewer
or by me; the Post-017 entry stands on its face.

## Applicator's own runs (closing record)

- `cargo fmt`: applied, exit 0; `cargo fmt --check` clean
  afterwards.
- `sh check.sh`: **all gates green**, exit 0 — format check,
  34 lib tests (including the extended `sheet_cap_holds`),
  6 primitives tests, round-trip / multi-sheet / sealed /
  template suites, clippy `-D warnings`, docs.
- Byte-identity: the applicator changes cannot move a byte.
  The `sheet_cap` change only enlarges the cap (6x per char
  is never below the escaped length, and never below the old
  `name.len()` price since a char is at most 4 UTF-8 bytes),
  so the oracle's `with_capacity` grows without changing its
  contents and the `archive_fits` projection stays
  conservative; the extraction is a verbatim move pinned by
  the unchanged primitives and round-trip tests in the gate.
  Existing-kind bytes therefore stand as the reviewer
  `cmp`-verified them at 200k (ints 10470009, floats
  10870010, null 10470071, mixed 17047851, three-sheet
  21338118), and no emitted spelling was edited.
- Measurement state: unchanged from Post-017. Retained
  capacities rest on `sheet_hint`, which I did not touch
  (strings 17003421/16381122 1.04x, bools 11403419/10781120
  1.06x as reproduced by the reviewer); the oracle capacity
  for escapable header names is now larger by the corrected
  bound, which is the point of the proof fix, and ordinary
  names gain only 5x their length per column. No bench was
  re-taken and no speed is claimed for these changes.
- Everything is left uncommitted, as instructed.
