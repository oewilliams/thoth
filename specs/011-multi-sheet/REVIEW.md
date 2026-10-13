# Cycle 011 — Review (multi-sheet)

Reviewer: independent review agent. Producer code was read in full and
probed; no source was edited by this review. The only file written is
this `REVIEW.md`. Everything is left uncommitted.

Tree snapshot before review: `git diff | shasum` =
`28eeaad015cda65bde64d7de99dcf0e9fc74bfae`; `git status` showed the
seven producer-modified tracked files (`DESIGN.md`, `README.md`,
`TESTING.md`, `benches/fill_write.rs`, `src/static_parts.rs`,
`src/xlsx.rs`, `tests/common/mod.rs`) plus untracked `tests/multi_sheet.rs`
and `specs/011-multi-sheet/`. After review the same snapshot was
re-taken and matched (see "Tree integrity" below).

## Build and test evidence

- `CARGO_HOME=/tmp/cargo-home sh check.sh` — **green** (`cargo fmt
  --check`, tests, clippy `-D warnings`, docs). 30 unit tests + 1
  round-trip + 1 multi-sheet + 4 sealed tests pass; `cargo fmt --check`
  separately confirmed clean.
- Independent probes (scratch crates in `/tmp/probe_head` against a
  `git archive HEAD` extraction and `/tmp/probe_cur` against the
  working tree, release profile):
  - Single-sheet `data` byte-identity vs HEAD: small example archive
    `len=3501 crc=C47A1A82` on both trees, `cmp` identical; 200k ints
    `10470009`/`966E8B4D` and 200k floats `10870010`/`6668CD1F` match
    on both; wrapper vs direct `build_workbook(&[Sheet{name:"data",..}])`
    equal.
  - Fragment cat-identity: parts extracted from the built archive
    (both trees) — `[Content_Types].xml` 680/`CC79080A`,
    `xl/workbook.xml` 282/`D4738FA2`, `xl/_rels/workbook.xml.rels`
    424/`8A9642E0`, `_rels/.rels` 296/`82C75906`, `xl/styles.xml`
    708/`24A3B074` — the HEAD whole-part pins reproduced exactly.
  - Sheet-name refusals: every class below probed (see R3), including
    31-char ASCII and multibyte (`é`×31 ok, ×32 refused by
    `chars().count()`), interior apostrophe `O'Brien` accepted,
    `History ` (trailing space, never trimmed) accepted.
  - Ragged in second sheet: `sheet "Second": column "b": ragged
    length 1, want 2`; wrapper spelling `sheet "data": …`; infinite in
    second sheet and empty-columns in second sheet likewise wrapped.
  - Workbook order/escaping probe (sheets `Zeta`, `A&B`, `A<B"C`):
    `xl/workbook.xml` lists them in declaration order with
    `A&amp;B`, `A&lt;B&quot;C`, `sheetId`/`r:id` = 1/2/3; rels carry
    `rId1..rId3` worksheets plus styles at `rId4`; archive entries are
    the 5 statics then `sheet1..3.xml` (count 8 = 5 + 3).
  - Multi-sheet bench recipe size: `build_workbook` over
    ints-20k/floats-20k/header-only gives 2018110 bytes, matching the
    Post-011 entry's recorded size; a 12-sheet build assembles.

## Verdicts per requirement

- **R1 — Sheet API, wrappers keep today's bytes: PASS.**
  `Sheet { pub name: String, pub columns: &'a [(Column,
  ColumnData<'a>)] }` at [src/xlsx.rs](/Users/oliverwilliams/Documents/thoth/src/xlsx.rs:49)
  matches the spec spelling; `build_workbook`/`write_workbook_file`
  take `&[Sheet<'_>]` ([src/xlsx.rs](/Users/oliverwilliams/Documents/thoth/src/xlsx.rs:568),
  [src/xlsx.rs](/Users/oliverwilliams/Documents/thoth/src/xlsx.rs:657));
  `build_xlsx`/`write_xlsx_file` are one-sheet wrappers naming the
  sheet `data` ([src/xlsx.rs](/Users/oliverwilliams/Documents/thoth/src/xlsx.rs:645),
  [src/xlsx.rs](/Users/oliverwilliams/Documents/thoth/src/xlsx.rs:684)).
  Byte-identity and wrapper-equality probed (above); existing callers,
  tests, and benches compile unmodified (gate green).

- **R2 — Workbook parts are fragment concatenations, pinned: PASS.**
  `RELS`/`STYLES` stay whole constants; the three sheet-naming parts
  are cut at the spec joints in [src/static_parts.rs](/Users/oliverwilliams/Documents/thoth/src/static_parts.rs:10)
  (content-types head ends at the workbook override; workbook head ends
  at `<sheets>`; rels head ends at `<Relationships …>`, styles rel at
  count + 1). Assembly is fragment `push_str` + `push_usize`, no
  `format!` on the assembly path ([src/xlsx.rs](/Users/oliverwilliams/Documents/thoth/src/xlsx.rs:331)).
  `static_parts_are_pinned` replaces the whole-part pins with 17
  fragment pins plus cat-identity at 680/`0xCC79080A`,
  282/`0xD4738FA2`, 424/`0x8A9642E0` ([src/xlsx.rs](/Users/oliverwilliams/Documents/thoth/src/xlsx.rs:815)),
  independently reproduced from archive bytes (above). Capacity
  arithmetic sizes each part once from fragment lengths +
  escaped-name lengths + `digit_width` ([src/xlsx.rs](/Users/oliverwilliams/Documents/thoth/src/xlsx.rs:322),
  [src/xlsx.rs](/Users/oliverwilliams/Documents/thoth/src/xlsx.rs:348),
  [src/xlsx.rs](/Users/oliverwilliams/Documents/thoth/src/xlsx.rs:385))
  and `workbook_parts_cap_holds` pins `len == cap` at 1, 3, and 12
  sheets plus an escaped name, the `sheet_cap_holds` pattern
  ([src/xlsx.rs](/Users/oliverwilliams/Documents/thoth/src/xlsx.rs:897));
  `escaped_len` matches `escape_into` spellings (5/4/4/6/6).

- **R3 — Sheet-name validation at the boundary: PASS.**
  `validate_sheet_names` runs once in `build_workbook` before any
  sheet is emitted ([src/xlsx.rs](/Users/oliverwilliams/Documents/thoth/src/xlsx.rs:431),
  called at [src/xlsx.rs](/Users/oliverwilliams/Documents/thoth/src/xlsx.rs:569)):
  empty, >31 `chars().count()`, `[ ] : * ? / \` and control chars,
  leading/trailing apostrophe, reserved `History`
  (case-insensitive), and case-insensitive duplicates each refuse via
  a dedicated `XlsxError` variant whose message contains the
  offending name ([src/xlsx.rs](/Users/oliverwilliams/Documents/thoth/src/xlsx.rs:55));
  empty slice is `NoSheets` → `no sheets declared`. One unit test per
  class asserts the name appears ([src/xlsx.rs](/Users/oliverwilliams/Documents/thoth/src/xlsx.rs:985));
  all classes independently probed, including multibyte limits and
  the never-trimmed `History ` case. (Minor test weakness, not a
  defect: the empty-name assertion `contains("")` at
  [src/xlsx.rs](/Users/oliverwilliams/Documents/thoth/src/xlsx.rs:993)
  is vacuous; the companion `contains("empty")` carries the check.)

- **R4 — Per-sheet errors name the sheet: PASS.** All `sheet_xml`
  refusals inside a workbook wrap in `InSheet`, displayed as
  `sheet "{sheet}": {inner}`, preserving every pinned substring
  ([src/xlsx.rs](/Users/oliverwilliams/Documents/thoth/src/xlsx.rs:76),
  [src/xlsx.rs](/Users/oliverwilliams/Documents/thoth/src/xlsx.rs:572)).
  Probed: ragged/infinite/empty-columns in the second sheet all name
  `Second` plus the column/condition; the single-sheet wrapper names
  `data` and retains `b`/`row 3`. The two-sheet unit test pins sheet
  and column ([src/xlsx.rs](/Users/oliverwilliams/Documents/thoth/src/xlsx.rs:1045));
  existing error tests pass unmodified (`tests/roundtrip.rs`,
  `tests/sealed.rs` untouched in the diff — the only test-code change
  outside the R2 pin replacement is the additive reader helpers, so
  no R4-forced edit listing is owed).

- **R5 — Multi-sheet round-trip through the independent reader:
  PASS.** `read_sheet_values_by_index` sits beside `read_sheet_values`,
  which remains the sheet-1 spelling ([tests/common/mod.rs](/Users/oliverwilliams/Documents/thoth/tests/common/mod.rs:265)).
  [tests/multi_sheet.rs](/Users/oliverwilliams/Documents/thoth/tests/multi_sheet.rs:19)
  writes three sheets (2 cols/2 rows mixed kinds; 1 col/3 rows named
  `A&B`; 3-col header-only), reads each back per index, checks the
  sheet-1 spelling agrees, asserts each sheet's bytes embed verbatim,
  and asserts workbook names appear in declaration order with
  `A&amp;B` escaping. Negative store examples stay single-sheet;
  `tests/roundtrip.rs` is green unmodified.

- **R6 — Byte-identity and gate for everything that existed: PASS.**
  `sh check.sh` green (above); `src/zip.rs` is byte-identical to HEAD
  (sha `062b4557…` both sides; the `ZipWriter::new(parts.len())` count
  is the 5 + N set by construction at
  [src/xlsx.rs](/Users/oliverwilliams/Documents/thoth/src/xlsx.rs:587)).
  Golden, verbatim-embed, `sheet_cap_holds`, emit spelling, and the
  four sealed locks all pass unmodified; `src/emit.rs`,
  `src/contract.rs`, `Cargo.toml` untouched.

- **R7 — Zip32 ceiling re-checked for the grown entry set: PASS.**
  `archive_fits` runs over the assembled 5 + N part lengths
  ([src/xlsx.rs](/Users/oliverwilliams/Documents/thoth/src/xlsx.rs:611));
  the cast-for-cast comment is updated to the workbook entry set and
  states that sheet-name bytes appear only inside the assembled
  workbook parts as data lengths, never in zip names
  ([src/xlsx.rs](/Users/oliverwilliams/Documents/thoth/src/xlsx.rs:602)),
  matching the `archive_fits` cast mapping
  ([src/xlsx.rs](/Users/oliverwilliams/Documents/thoth/src/xlsx.rs:248)).
  Existing `archive_fits` tests pass; the new
  `archive_fits_many_sheets_overflow_via_entry_count` isolates the
  count ceiling (fits at 65535 small entries, refuses at 65536)
  ([src/xlsx.rs](/Users/oliverwilliams/Documents/thoth/src/xlsx.rs:762)).

- **R8 — No-regression bench, and docs catch up: PASS.**
  The Post-011 entry in [TESTING.md](/Users/oliverwilliams/Documents/thoth/TESTING.md:591)
  records machine (Darwin 25.6.0 arm64, Apple M4), bench profile, and
  commit `fe0e896` (= HEAD). Its one claimed pair, floats 200k
  11.671/11.765ms, satisfies the spell-agreement rule (0.81% apart,
  CI half-widths 1.6%/1.9%, severe outliers 9%/8%) and sits within
  noise of the Post-010 means (+0.8%/+0.4% vs 11.580/11.714ms). Ints,
  floor, `fill_write`, and the new `multi_sheet/three_sheet_20000`
  bench ([benches/fill_write.rs](/Users/oliverwilliams/Documents/thoth/benches/fill_write.rs:149),
  sheets/columns built once outside the clock, timed section exactly
  `write_workbook_file`) claim **no** numbers: the gaps, discarded
  passes, and outlier shares are recorded to the Post-006/007/009
  standard, and the isolated ints read (6.805ms, CI [6.500, 7.178])
  overlaps both Post-010 means, so no regression is indicted and none
  is hidden; the multi-sheet baseline is explicitly left for 012 to
  re-take. `DESIGN.md` Scope and `README.md` Shape no longer claim
  one sheet (grep: no remaining `one sheet` claim in either file).
  This review did not re-run criterion: the recorded reads were
  checked against the protocol arithmetic, and the bench recipe's
  output size was independently reproduced (2018110 bytes).

## Required changes for the applicator

None. R1–R8 all pass on inspected code, a green gate, and independent
probes. Two non-blocking observations for the record, neither owed
this cycle: the vacuous empty-name assertion noted under R3, and
`src/zip.rs`'s module doc still naming `build_xlsx` as the ingress
owner (zip.rs is scoped untouched this cycle; `build_workbook` owns
the entry set now — worth one line whenever zip.rs is next opened).

## Tree integrity

After writing only this file, `git diff | shasum` and `git status`
were re-checked against the pre-review snapshot: the tracked diff is
byte-identical and no file outside `specs/011-multi-sheet/REVIEW.md`
was created or modified by this review. Probes live in `/tmp` only.
Everything remains uncommitted for the applicator.
