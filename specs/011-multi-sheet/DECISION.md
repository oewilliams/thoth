# Cycle 011 — Decision (multi-sheet)

Applicator: independent applicator agent. Read `SPEC.md`, `REVIEW.md`,
`STYLE.md`, `DESIGN.md`, and `TESTING.md` in full before acting. No
source, test, bench, or doc file was edited by this application: the
reviewer required no changes and the applicator's own verification
found no defect, so there was nothing to apply. Everything is left
uncommitted.

## What shipped (producer work, as reviewed)

- **R1 — Sheet API.** `Sheet { name: String, columns: &[(Column,
  ColumnData)] }`; `build_workbook` / `write_workbook_file` take
  `&[Sheet<'_>]`; `build_xlsx` / `write_xlsx_file` remain as one-sheet
  wrappers naming the sheet `data`, so existing callers, tests, and
  benches compile unmodified.
- **R2 — Fragment-cat workbook parts.** The three sheet-naming static
  parts are head/per-sheet/tail `const &str` fragment concatenations
  in `src/static_parts.rs` (only the escaped name and the index digits
  vary; `push_str` + `push_usize`, no `format!` on the assembly path);
  `RELS`/`STYLES` stay whole. `static_parts_are_pinned` now pins 17
  fragments plus cat-identity at the old whole-part lengths/CRCs
  (680/`0xCC79080A`, 282/`0xD4738FA2`, 424/`0x8A9642E0`), and
  `workbook_parts_cap_holds` pins `len == cap`.
- **R3 — Sheet-name validation.** Checked once in `build_workbook`
  before any emission: empty, >31 `chars().count()`, `[ ] : * ? / \`
  or control characters, leading/trailing apostrophe, reserved
  `History` (case-insensitive), and case-insensitive duplicates each
  refuse via a dedicated `XlsxError` variant naming the offending
  sheet; an empty sheet slice refuses as `NoSheets`.
- **R4 — Per-sheet errors.** Workbook refusals wrap in `InSheet`
  (`sheet "{sheet}": {inner}`), preserving every previously pinned
  substring; the single-sheet wrappers therefore name sheet `data`.
- **R5 — Round-trip.** `read_sheet_values_by_index` beside the
  sheet-1 spelling in `tests/common/mod.rs`; `tests/multi_sheet.rs`
  round-trips three sheets of differing shape (one `A&B`-escaped,
  one header-only) and asserts workbook names in declaration order.
- **R6 — Gate.** Full gate green; `src/zip.rs` byte-identical to
  HEAD (the `ZipWriter::new(parts.len())` count is 5 + N by
  construction).
- **R7 — Zip32 ceiling.** `archive_fits` runs over the assembled
  5 + N part lengths with the cast-for-cast comment updated; new
  test isolates overflow via entry count alone.
- **R8 — Bench and docs.** Post-011 entry in `TESTING.md` records
  one spell-agreeing claimed pair (floats 200k 11.671/11.765 ms,
  within noise of the Post-010 means) and documents the remaining
  groups, including the new `multi_sheet/three_sheet_20000` bench
  example, as unclaimed pending 012 re-takes. `DESIGN.md` Scope and
  `README.md` Shape no longer claim one sheet.

## Review verdict

Reviewer passed R1–R8 with no required changes, on inspected code, a
green gate, and independent probes (HEAD-vs-tree byte-identity at
small and 200k scales, fragment cat-identity from archive bytes,
every name-refusal class, second-sheet error wrapping, workbook
order/escaping, and the multi-sheet bench recipe size). Two
non-blocking observations were recorded for the record, neither owed
this cycle: the vacuous empty-name `contains("")` assertion (the
companion `contains("empty")` carries the check), and `src/zip.rs`'s
module doc still naming `build_xlsx` as the ingress owner — to be
corrected whenever `zip.rs` is next opened, since it is scoped
untouched this cycle. Both are accepted as noted; no action taken.

## Applicator verification (independent)

- **Gate:** `sh check.sh` run in this session — green: `cargo fmt
  --check` clean, 30 unit tests + 1 round-trip + 1 multi-sheet + 4
  sealed tests pass, `cargo clippy --locked --all-targets -- -D
  warnings` clean, docs built. Matches the reviewer's reported run.
- **Byte-identity spot-check:** scratch probe crates in `/tmp`
  (`probe_head` against a `git archive HEAD` extraction, `probe_cur`
  against the working tree, release profile with
  `overflow-checks = true`, `codegen-units = 1`), each building via
  the `build_xlsx` wrapper a small mixed int/float sheet (including
  `i64::MIN` and NaN cells) and a 20,000-row int sheet. HEAD and
  working tree agree exactly: small `len=3677 crc=FF2AA618`, wide
  `len=1018055 crc=672BCC4C`, and `cmp` over the concatenated bytes
  is clean. This independently confirms the single-sheet `data`
  byte-identity claim on an applicator-chosen recipe (the reviewer's
  3501-byte recipe and 200k figures stand as their own evidence).
- **Disposition:** no defect found, no disagreement with the
  reviewer, no code changed. The cycle is complete as reviewed;
  012 may start from this tree once committed.
