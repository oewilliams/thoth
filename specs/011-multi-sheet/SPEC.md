# Cycle 011 — multi-sheet (named sheets, per-sheet columns, fragment-cat workbook parts)

Phase 1 of 3 (011 multi-sheet → 012 phase bench → 013 fused emit).
011 lands on the current foundation unchanged in shape: every sheet is
still built as its own `Vec<u8>` by the existing `sheet_xml` emitter,
and the archive is still assembled in memory by `ZipWriter`. No pass
is fused, no sink changes, no streaming. 012 measures the passes;
013 removes the ones the numbers indict.

## Background

The single sheet was a scoping error, now corrected by the caller:
a workbook holds multiple named sheets, and each sheet carries its
own columns (its own template, in the sense `DESIGN.md` Templates
already allows — statically or dynamically shaped per sheet; the
Rust shape of a template is still deliberately not locked, and this
cycle does not lock it: a sheet here is a name plus the existing
borrowed column set).

Current state, inspected: `src/xlsx.rs:303-353` `build_xlsx` stages
exactly six parts, with `xl/workbook.xml`, `xl/_rels/workbook.xml.rels`,
and `[Content_Types].xml` frozen as whole-file constants in
`src/static_parts.rs:4-14` (workbook names the one sheet `data`,
rels carry `rId1` worksheet + `rId2` styles, content-types carries
the one sheet override). `RELS` (`_rels/.rels`) and `STYLES`
(`xl/styles.xml`) name no sheet and stay frozen whole.

Correction to the obvious worry: the three dynamic parts do **not**
become generated XML in any general sense. Each becomes a
**concatenation of frozen fragments** — head, one repeated per-sheet
fragment, tail — with only the escaped sheet name and the sheet
index digits varying between the fragments. The fragments are
`const &str` in `static_parts.rs`, cut so that concatenating them
for the single sheet `data` reproduces today's golden bytes
exactly. The per-sheet loop is `push_str` of a fragment, the escaped
name, `push_usize` digits (the existing `src/emit.rs:57-60`
spelling, formatted once per fragment, never `format!` on the hot
assembly path) — the same micro-opt register the shipped code
already works in: `src/zip.rs` cycle 004 (buffer sized once from a
running total, `push_u16`/`push_u32` leaving no `bl` on the push
paths, no duplicate scan in `add`) and `src/xlsx.rs` cycle 005
(`sheet_cap` sizing with the `sheet_cap_holds` no-realloc proof,
row digits formatted once per row at `src/xlsx.rs:268-275`). The
workbook parts are kilobytes; their cost is a handful of fragment
copies per sheet, and the spec requires it stay that way.

Binding references: `STYLE.md` (rule 1 caller-backed scope, rule 3
explicit spellings — conversions and targets on the line, rule 7
the oracle stays, rule 8 wrong is worse than stopped, rule 9
measurement is definition), `DESIGN.md` (Ingress, Templates,
sealed writes), `TESTING.md` (bench protocol, spell-agreement rule
from cycle 010).

## Requirements (each item testable)

- **R1 — Sheet API, single-sheet wrappers keep today's bytes.**
  A public sheet type carries a name and one borrowed column set:
  `pub struct Sheet<'a> { pub name: String, pub columns:
  &'a [(Column, ColumnData<'a>)] }` (field names may differ only if
  the reviewer accepts the alternative as clearer on the line).
  Workbook entry points take `&[Sheet<'_>]`: `build_workbook` and
  `write_workbook_file` (names likewise reviewer-confirmed). The
  existing `build_xlsx` / `write_xlsx_file` keep their signatures
  and become one-sheet wrappers using the name `data`, so every
  existing caller, test, and bench compiles unmodified and a
  one-sheet workbook named `data` is byte-identical to today —
  including `xl/workbook.xml` naming `data`. Test: existing suite
  green unmodified (R6); a probe comparing wrapper output against
  direct `build_workbook(&[Sheet { name: "data", .. }])` shows
  identical bytes.
- **R2 — Workbook parts are fragment concatenations, pinned.**
  `src/static_parts.rs` keeps `RELS` and `STYLES` as whole
  constants and replaces the three sheet-naming constants with
  fragment constants, cut at these joints (exact constant names
  are the producer's, the joints are not):
  - content-types: head ending at the workbook override; per sheet
    `<Override PartName="/xl/worksheets/sheet` + index digits +
    `.xml" …worksheet+xml"/>`; tail holding the styles override
    and `</Types>`.
  - workbook: head ending at `<sheets>`; per sheet `<sheet name="`
    + escaped name + `" sheetId="` + digits + `" r:id="rId` +
    digits + `"/>`; tail `</sheets></workbook>`.
  - workbook rels: head ending at `<Relationships …>`; per sheet
    `<Relationship Id="rId` + digits + `" …/worksheet"
    Target="worksheets/sheet` + digits + `.xml"/>`; then the
    styles relationship at `rId` (sheet count + 1); tail
    `</Relationships>`.
  Sheet files are `xl/worksheets/sheet{index}.xml`, 1-based in
  declaration order; `sheetId` and `r:id` are the same 1-based
  index. Concatenation for one sheet named `data` must reproduce
  the three current constants byte for byte: the pin test replaces
  the three old whole-part pins with fragment pins **plus** a
  cat-identity assertion pinning the old lengths and CRCs
  (680/`0xCC79080A` content-types assembled, 282/`0xD4738FA2`
  workbook, 424/`0x8A9642E0` workbook rels — verify against
  `static_parts_are_pinned` at HEAD before quoting). Assembly
  sizes each part once from fragment lengths plus escaped-name
  lengths plus digit widths (no realloc on the assembly path;
  the producer shows the capacity arithmetic, the reviewer checks
  it the way `sheet_cap_holds` checks `sheet_cap`). Test: pin and
  cat-identity tests green; `cargo fmt --check` clean.
- **R3 — Sheet-name validation at the boundary, refused loudly.**
  Checked once in `build_workbook` before any sheet is emitted
  (never per cell, never repaired, never trimmed — STYLE rule 8):
  empty name refused; more than 31 characters (`chars().count()`,
  Excel's limit is characters, not bytes) refused; any of
  `[ ] : * ? / \` or a control character refused; a leading or
  trailing apostrophe refused; the reserved name `History`
  (case-insensitive) refused; duplicate names under
  case-insensitive comparison refused. Each refusal is a new
  `XlsxError` variant whose message names the offending sheet
  name and the rule it broke (wording producer's, content pinned
  by test: the message contains the name). An empty sheet slice
  is refused as its own variant (`no sheets declared` wording).
  Test: one unit test per refusal class, each asserting the
  message contains the offending name; a 31-char name and a name
  needing XML escape (e.g. `A&B`) round-trip (R5).
- **R4 — Per-sheet errors name the sheet.** The existing
  per-column refusals (`Ragged`, `Infinite`, `EmptyColumns`,
  `TooManyRows`, `TooManyColumns`) gain the sheet name in their
  message when raised inside a workbook — by a wrapping variant
  or by added fields, producer's shape, one spelling everywhere:
  the resulting message contains the sheet name **and** retains
  every substring today's tests pin (column name, `row 3`, row
  counts). Single-sheet wrappers therefore produce messages
  naming sheet `data`; today's pinned substrings in
  `tests/common` (`ragged` → `b`, `infinite` → `px`) and in
  `src/xlsx.rs` tests still match. Test: a two-sheet probe with
  the ragged column in the second sheet asserts both sheet and
  column names appear; existing error tests green unmodified.
- **R5 — Multi-sheet round-trip through the independent reader.**
  `tests/common/mod.rs` gains a by-index (or by-part-name) sheet
  reader beside `read_sheet_values`, which keeps working as the
  sheet-1 spelling. A new integration test writes a workbook of
  at least three sheets — differing column counts, kinds, and row
  counts per sheet, one header-only sheet, one sheet name needing
  escape — reads every sheet back through the store reader, and
  compares values per sheet; it also parses `xl/workbook.xml`
  from the archive and asserts the sheet names appear in
  declaration order (escaped spelling where required). Negative
  store examples stay single-sheet. Test: the new test green;
  `tests/roundtrip.rs` green unmodified, including the
  per-example verbatim-embed check (`sheet_xml` bytes appear
  verbatim in the archive — still true per sheet, since sheets
  are staged whole this cycle).
- **R6 — Byte-identity and gate for everything that existed.**
  Full gate passes with no existing test edited except the
  `static_parts_are_pinned` replacement in R2 and any test the
  R4 error shape forces (each such edit listed in `REVIEW.md`
  with the substring preserved): round-trip and verbatim-embed,
  `golden_small_archive_is_byte_identical`, `sheet_cap_holds`,
  emit spelling tests, `tests/sealed.rs` locks. `zip.rs` is
  untouched except — only if the entry-count construction needs
  it — the `ZipWriter::new(entry_count)` call site count
  (5 + sheet count); the writer's contract (ingress-validated,
  sized once, no duplicate scan) does not change. Test:
  `sh check.sh` green; `git diff --stat` shows `src/zip.rs`
  empty or call-site-only.
- **R7 — Zip32 ceiling re-checked for the grown entry set.**
  `archive_fits` now runs over 5 + N entries with the assembled
  part lengths; the cast-for-cast comment in `build_xlsx` is
  updated to the workbook entry set, and sheet-name lengths no
  longer appear in zip names (sheet parts are `sheet{index}.xml`)
  but do appear in the assembled workbook parts' lengths, which
  the check covers as data lengths. The existing `archive_fits`
  unit tests stay green; one new test stages sizes for a
  many-sheet set that overflows only via entry count or central
  length. Test: `archive_fits` tests green including the new one.
- **R8 — No-regression bench, and docs catch up.** Re-take
  `fill_write` examples plus `cells` and `floor` groups per the
  `TESTING.md` protocol (spell-agreement rule: two agreeing
  quiet-spell reads before any number is recorded). Single-sheet
  path within noise of the Post-010 recorded means at 200k
  (floats 11.580/11.714ms, ints 6.578/6.646ms — re-verify at HEAD
  before quoting); any regression beyond noise fails the cycle.
  One new bench example exercises a 3-sheet workbook at bench
  scale so 012 inherits a multi-sheet baseline (recipe in
  `tests/common` or bench-local, producer's placement, named in
  `REVIEW.md`). `DESIGN.md` Scope and `README.md` Shape lose the
  one-sheet wording in the same commit (multi-sheet, per-sheet
  columns, sealed writes unchanged); `TESTING.md` gains the
  Post-011 entry. Test: Post-011 entry holds agreeing reads,
  machine, profile, commit; docs diff shows no remaining
  `one sheet` claim.

## Scoped files

`specs/011-multi-sheet/SPEC.md` (this file) now; on `go`:
`src/xlsx.rs` (Sheet API, workbook assembly, validation, error
shape), `src/static_parts.rs` (fragment constants, `RELS`/`STYLES`
untouched), `src/contract.rs` only if the reviewer rules the sheet
type belongs beside `Column` (default: `xlsx.rs`, where ingress
lives), `tests/common/mod.rs` (by-index reader), one new
integration test file for R5, `benches/fill_write.rs` and
`tests/common` recipes for the R8 bench example, `DESIGN.md`,
`README.md`, `TESTING.md` (Post-011 entry), `specs/011-multi-sheet/`
records by role. Untouched: `src/emit.rs`, `src/zip.rs` (R6
call-site exception only), `Cargo.toml`, `Cargo.lock`, `STYLE.md`.
No new dependencies. Probes in `/tmp` scratch.

## Acceptance

- Gate green (`cargo fmt --check`, `sh check.sh`).
- One-sheet `data` workbook byte-identical to HEAD output;
  fragment cat-identity pins the three old parts.
- Three-sheet workbook round-trips per sheet with names in order;
  every name-refusal class pinned by test.
- All 200k benches within noise on agreeing reads; Post-011 entry
  recorded; `DESIGN.md`/`README.md` no longer claim one sheet.
- Reviewer verdict per item in `REVIEW.md`; applicator reasoning
  in `DECISION.md`; everything left uncommitted.

## Boundaries (not this cycle)

- No pass fusion, no sink change, no direct-to-file emission
  (012 measures, 013 fuses — in that order, and only what 012
  indicts).
- No template trait or macro: a sheet is a name plus the existing
  column set. The static/dynamic template split stays un-locked.
- No new primitives (strings/bools still wait for their caller),
  no styles/palette change, no deflate, no second reader oracle.
- No parallel sheet emission: sheets are built sequentially in
  declaration order until a bench shows the sequential build,
  not the passes, is the cost.
- Sheet-name validity is Excel's rule set as listed in R3 and
  nothing more — no locale folding tables, no name normalisation.

## Workflow (on `go`)

Producer, reviewer, applicator as three independent agents, one
role each, per `specs/README.md`: the producer gets this SPEC plus
`STYLE.md`, `DESIGN.md`, `TESTING.md`, and nothing else; the
reviewer reads code and spec, builds, runs probes and tests, and
creates only `REVIEW.md` (the tree byte-identical before and
after outside scoped files, verified by diff); the applicator
applies required changes and writes `DECISION.md`, leaving
everything uncommitted. No agent reviews its own work. 012 does
not start until 011 is applicated and green.
