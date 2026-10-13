# Cycle 015 — Decision (template contract)

Applicator, after the reviewer. Reviewer verdict: conditional pass
with required changes RC1–RC4 plus applicator-stage records. I agree
with all four required changes; no disagreement is recorded, because
each names a real gap against the SPEC on inspected evidence:

- RC1: kind is declared twice (`Column.kind` and the `ColumnData`
  variant) and only the data variant is read, so a disagreement is
  silently emitted. R1 makes declaration the contract's first
  clause, so `validate_template` must refuse the disagreement.
- RC2: `DynamicSheet::unknown_columns` declares the column
  dimension unknown, but `sheet_hint` re-derives the witnessed
  columns and returns the known-spelling hint. Either the hint
  honours the named unknown (`None` → builder block growth, as for
  rows) or the contract narrows to rows-only. I take the first
  branch: it keeps the declared `UnknownDimension::Columns`
  spelling honest and costs only capacity, never bytes.
- RC3: the in-tree unknown-rows tests build 2–3 rows, far under
  the initial `ARENA_BLOCK` reservation, so no gated test forces
  arena growth on the unknown path. A large in-tree byte-identity
  build is required.
- RC4: the `src/xlsx.rs` rewrite trimmed the `archive_fits`
  cast-cover enumeration and sizing rationale that Post-005/014
  recorded as load-bearing. STYLE holds docs and code to the
  same bar, so the rationale is restored (adapted to the contract
  spellings now in `src/contract.rs`), not waived.

## What changed

- **RC1** (`src/contract.rs`, `src/xlsx.rs`): `ColumnView` now
  carries the declared `kind` (from `Column.kind` for
  `DynamicSheet`, from `StaticColumn::kind()` for `StaticSheet`).
  `validate_template` refuses a disagreement before any emission
  with the new `XlsxError::KindMismatch` ("declared kind f64 does
  not match data kind i64"). After that boundary check the drivers
  still key off the data variant, which now provably coincides
  with the declaration. Test: `kind_mismatch_is_refused_at_the_boundary`
  (oracle, `build_workbook`, and the `sheet_xml` compat spelling).
- **RC2** (`src/contract.rs:408`): `sheet_hint` returns `None`
  when either dimension is unknown (`rows()?` plus `columns()?`),
  so unknown columns answer with `ARENA_BLOCK` block growth in
  `build_workbook`, exactly as unknown rows do; the witnessed
  columns are no longer re-derived to spell a hint. Test:
  `unknown_columns_bytes_match_known_and_hint_is_none` (hint
  `None`, sheet and archive bytes identical to the known spelling).
- **RC3** (`tests/template.rs`): `unknown_rows_growth_bytes_match_known_shape`
  builds 5,000 rows (archive well past the 8,192-byte `ARENA_BLOCK`
  initial reservation) through `build_workbook` and asserts
  byte-identity with the known spelling, forcing arena growth in
  the gate rather than in a `/tmp` probe.
- **RC4** (`src/xlsx.rs`, `src/contract.rs`): restored the trimmed
  rationale — the Zip32 constant derivations, the `archive_fits`
  cast-cover enumeration (adapted: `sheet_cap` now lives in
  `crate::contract`), the fold exactness note, the per-cast
  comments at `patch_local_header` / `central_start` /
  `central_len`, the ceiling and reservation comments in
  `build_workbook`, and the `sheet_cap` widest-value derivation
  (20 `I64` / 327 `F64`, row digits cap at 7) in `contract.rs`.
- **Signature record (R5)**: `build_workbook` and
  `write_workbook_file` are generic over `SheetTemplate`; `Sheet`
  is the `DynamicSheet` alias, constructible only via
  `DynamicSheet::known` / `unknown_rows` / `unknown_columns`
  (private `shape` field, no struct literal). Every in-tree caller
  (library, examples, benches, tests) was migrated by the producer
  to the contract. The kept spellings `sheet_xml`, `build_xlsx`,
  and `write_xlsx_file` are one-line delegations through
  `DynamicSheet::known`; `write_xlsx_file` is uncalled in-tree and
  stands as a kept public spelling by delegation. No other public
  signature changed; `src/emit.rs` is untouched.

## DESIGN.md Templates wording

`DESIGN.md` Templates now reads as shipped in this cycle: the
`SheetTemplate` contract and its declaration order, kind-at-column
with the RC1 refusal, `ShapeKnowledge` / `UnknownDimension`, the
hint-or-named-unknown reservation (`sheet_hint` is `None` when
either dimension is unknown; `sheet_cap` is the proof, not the
reservation), the two shipped impls with their callers and the
generic builders, the compat delegations, monomorphisation with
the contract driving the row loop / letters / digits / per-row
CRC boundaries, and the Post-015 retained-capacity record with
the blank-heavy successor decision still named, not taken. The
stale sentences "No current caller declares an unknown dimension"
and "the exact Rust shape … is deliberately not locked yet" are
removed.

## Applicator's own runs (closing record)

- `cargo fmt` applied (needed one clippy-driven respelling:
  `columns()?` in `sheet_hint`), then `sh check.sh`:
  **all gates green** — format check, tests (33 lib tests plus
  round-trip, multi-sheet, 4 sealed, and 9 template tests — the
  6 producer tests plus the 3 applicator tests above), clippy
  `-D warnings`, docs.
- Byte-identity: `src/emit.rs` diff against HEAD is empty. My own
  `/tmp` probe (HEAD from a `git archive` extraction vs the
  current tree, `build_xlsx` at 200,000 rows) is `cmp`-identical
  on the ints recipe (10,470,009 bytes both sides, capacity
  12,203,418 both sides — matching the reviewer's and Post-015's
  numbers) and on a single-column null recipe (7,381,119 bytes,
  capacity 12,203,418, identical both sides). The RC changes touch
  no valid-sheet bytes: RC1 only refuses previously-misemitted
  declarations, RC2 changes only the unknown-columns reservation
  (capacity, never bytes), RC4 is comments only.
- Measurement state: unchanged from Post-015 and the reviewer's
  corroboration, and not re-taken here — known-shape capacities
  reproduce (ints 12,203,418 / 10,470,009 above); the null-heavy
  1.93x retention stands as recorded in `TESTING.md` Post-015 with
  the blank-heavy hint decision still the successor's; unknown
  columns now reserve by growth instead of the known-spelling
  hint, the honest price already priced for unknown rows (+6–12%
  at 200k in Post-015). No timing claim is made for these changes.
- Everything is left uncommitted, as instructed. Files edited by
  the applicator: `src/contract.rs`, `src/xlsx.rs`,
  `tests/template.rs`, `DESIGN.md`, and this `DECISION.md` —
  all inside the SPEC's scoped list.
