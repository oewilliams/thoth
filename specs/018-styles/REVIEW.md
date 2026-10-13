# Cycle 018 — Review

Reviewer, independent of the producer. Tree inspected: uncommitted
working tree at `3c1c266` plus the 018 changes (13 modified files,
new `examples/styled.rs`, `tests/styles.rs`). No source, test, or doc
file was edited by this review; the only file written is this one.
Gate, probes, and byte checks below were run by the reviewer.

## Verdict

Ship. No blocker and no major finding. Two minor findings (M1, M2),
neither changes emitted bytes. All seven SPEC requirements verified
against inspected source plus reviewer-run evidence.

## Byte-identity of the reviewed tree

`git status --porcelain` and hashes taken before any reviewer command
and re-taken after (see Evidence): `git diff HEAD` sha256
`73a2e90b7f97aecf…`, tracked-tree aggregate sha256
`a79c4697505db85f…`, untracked `examples/styled.rs`
`bcdb02d43fdb012b…`, `tests/styles.rs` `4eb175c4fd9ecc5e…`.
Source tree excluding this `REVIEW.md` is byte-identical before and
after the review; nothing was committed.

## Findings

### M1 (minor) — Non-identifier style tokens refuse without the grammar wording

`src/macros.rs:312`, `:357`, `:381`/`:388`/`:394`: the three entry
macros accept only `$style:ident` inside the brackets, so
`i64 [123] "t"` fails with `error: no rules expected \`123\`` instead
of the `column style grammar: …` message R5 requires for an unknown
style token. It is still a loud compile-time refusal, never a silent
plain column, and every identifier spelling refuses correctly (see
Evidence). Fix would be taking `tt` in the outer macros and letting
`__thoth_style`'s catch-all (`src/macros.rs:126-130`) name the
grammar. Producer's call; does not block.

### M2 (minor) — Integration xf assertions sample 6 of 14 entries

`tests/styles.rs:126-135` asserts xf entries 0, 1, the Red pair, and
the Grey pair only. The Amber/Blue/Purple xf entries are covered
indirectly: the frozen-part pin (`src/workbook_parts.rs:242`, len/CRC
over the whole part), the palette-order fill walk
(`tests/styles.rs:115-125`), the arithmetic test
(`tests/styles.rs:17-55`), and the emitter sweep over indices 1..=13
(`src/emit.rs:744-790`). No byte risk; naming it so the applicator
knows the stored-part assertion is a sample, not the full table.

### Observations (no action required)

- The index mapping is spelled twice: arithmetic in
  `Style::index` (`src/contract.rs:69-76`) and as a literal fragment
  table in `style_fragment` (`src/contract.rs:82-99`). They agree
  today and the agreement is pinned from both sides (arithmetic test
  plus sweep), but a future palette change must edit both.
- `sheet_cap` prices the widest fragment (7 bytes) for every styled
  column (`src/contract.rs:564`), so bold-only columns over-reserve
  by 1 byte per cell in the cap only. The cap is a proof, never the
  reservation (SPEC R4); `sheet_hint` prices the exact fragment
  (`src/contract.rs:621`). Correct as specced.

## Requirement checks (inspected file and line)

- **R1 — Style types.** `Colour` six variants in palette order and
  `Style { bold, colour }` with plain as `Style::default()`:
  `src/contract.rs:28-63`. `Column` (`:103-107`), every
  `StaticColumn` variant (`:316-337`), and `ColumnView` (`:229-234`)
  carry `style`; both `columns_view` impls gather it once
  (`:300-306`, `:402-408`). No validation added; boundary behaviour
  (`validate_template`, `:424-480`) untouched in kind.
- **R2 — Emission spelling.** `s` after `r`, before `t`, on all eight
  emitters; blanks in styled columns carry it; plain columns emit no
  `s`: `src/emit.rs:153-176` (i64), `:187-220` (f64), `:222-260`
  (bool), `:269-310` (str) and the byte twins `:384-570`. Exact
  spellings pinned at `src/emit.rs:598-624`. Fragment computed once
  per sheet per column: `src/contract.rs:663-667` and `:711-715`.
  Headers never touch the styled path (`:668-671`, `:716-719` call
  `header_cell`/`header_cell_bytes` unconditionally).
- **R3 — One spelling per driver pair; unstyled bytes unchanged.**
  Sweep over every kind valued/blank × indices 1..=13, oracle vs
  bytes: `src/emit.rs:744-790`. Reviewer HEAD comparison in Evidence.
- **R4 — Sizing.** `sheet_cap` prices ` s="13"` (7 bytes) per styled
  data cell including blanks (`src/contract.rs:561-599`);
  `sheet_hint` prices the exact fragment length (`:619-647`).
  `sheet_cap_holds` extended to styled columns of every kind at the
  widest index: `src/xlsx/tests.rs:316-367`. Hint exactness is
  corroborated by the styled/twin capacity difference equalling the
  `s` bytes exactly (TESTING.md Post-018 (b)).
- **R5 — Macro grammar.** `__thoth_style` accepts `bold`, one colour,
  colour+`bold` in either order; catch-all `compile_error!` names
  "column style grammar": `src/macros.rs:11-131`. Styled arms in both
  column helpers: `:165-192`, `:247-286`. Acceptance byte-tests
  (dynamic, `columns!`, static, both orders):
  `tests/styles.rs:145-221`. Three compile-fail doctests replace the
  old `[bold]` refusal: `src/macros.rs:338-354`. See M1 for the one
  refusal spelling that misses the wording.
- **R6 — Callers and round-trip.** `examples/styled.rs` (bold, red,
  green+bold, amber across all four kinds via the macro grammar);
  reviewer ran it (5419-byte file). Reserved `styled` store example
  added with styles in the declarations and joined to `all()`:
  `tests/common/mod.rs:386-459`. Values round-trip unchanged:
  `tests/styles.rs:57-67`. Stored-part `s` indices per cell (valued
  and blank), palette fills in order, counts, oracle equality:
  `tests/styles.rs:69-143`.
- **R7 — Measured record.** Post-018 entry present with machine,
  profile, commit, agreeing pairs, styled-vs-twin pricing, and the
  rejected first spelling named: `TESTING.md:1240-1323`. The 017
  null discrepancy stays unquoted (final-binary null pair recorded
  without a cross-binary delta, `:1288-1294`), as specced.

## Mandated spot checks

- **styles.xml validity and counts.** Reviewer parsed the frozen
  part (`src/static_parts.rs:50-51`) with an XML parser:
  well-formed; `fonts count="2"` = 2 `<font>`, `fills count="8"` = 8
  `<fill>`, `cellXfs count="14"` = 14 `<xf>`. Fills 0 `none`, 1
  `gray125` unmoved; fills 2..=7 solid `fgColor` in palette order
  `FFFF0000, FF00B050, FFFFC000, FF0070C0, FF7030A0, FFBFBFBF` —
  exactly the SPEC palette. Length 2235, CRC `0x76B2525A`,
  independently recomputed; pin at `src/workbook_parts.rs:242`
  matches. HEAD part was 708/`0x24A3B074` (recomputed from
  `git show HEAD:`), so the growth is exactly 1527 bytes.
- **Index arithmetic vs emitted `s`.** `Style::index`
  (`src/contract.rs:69-76`): plain none, bold 1, colour i → 2+2i /
  3+2i. The `cellXfs` table parsed from the part gives
  (fontId, fillId) `(0,0) (1,0) (0,2) (1,2) (0,3) (1,3) (0,4) (1,4)
  (0,5) (1,5) (0,6) (1,6) (0,7) (1,7)` — index n maps to palette
  entry n exactly, coloured entries carry `applyFill="1"`, bold
  entries `applyFont="1"`, bold-colour both. `style_fragment`
  (`:82-99`) spells the same indices; the sweep
  (`src/emit.rs:761-790`) proves both drivers emit them.
- **Header byte-identity.** `header_cell` (`src/emit.rs:41-49`) is
  unchanged from HEAD (same `t="inlineStr" s="1"` spelling, confirmed
  against `git show HEAD:src/emit.rs`) and is called for every sheet
  regardless of column styles; `tests/styles.rs:82-89` asserts the
  plain header spelling inside a fully styled sheet. The HEAD probe
  below also covers headers: the whole unstyled sheet oracle,
  headers included, is byte-identical.
- **Unstyled sheet byte-identity vs HEAD.** Reviewer probe
  (Evidence): identical oracle length and CRC on both trees, archive
  growth exactly the 1527 styles bytes. Producer's 200k six-recipe
  part-by-part comparison is recorded in TESTING.md Post-018 (a);
  the reviewer's smaller probe independently agrees.
- **sheet_cap truth on styled columns.** `sheet_cap_holds` runs
  styled columns of every kind at `s="13"` (Grey+bold, Purple,
  bold-only, Red) valued and blank (`src/xlsx/tests.rs:316-367`)
  and the full gate is green, so `len <= cap` holds there. The cap
  prices blanks (`src/contract.rs:585`, `:598` via `nrows`).
- **Macro refusals.** Reviewer compiled five refused spellings
  against the crate: `[pink]`, `[red, blue]`, `[bold, bold]`, `[]`,
  `[bold, red, blue]` — all fail with `column style grammar: …`
  (the `[]` case via the helper catch-all,
  `src/macros.rs:193-197`/`:287-291`). Only the non-identifier
  token in M1 misses the wording.
- **Hint pricing.** `sheet_hint` adds `style_fragment(style).len()`
  per data cell, exact spelling per column including blanks
  (`src/contract.rs:619-647`); Post-018 (b) retained-capacity
  figures show the styled/twin capacity difference is exactly the
  4,800,000 `s` bytes, so the hint is exact, not padded.
- **STYLE rule compliance.** Files over the 700-line tripwire carry
  the written annotation the rule requires: `src/contract.rs:6-9`
  (746 lines) and `src/emit.rs:5-8` (806 lines), each arguing the
  minimalism/performance benefit. New comments are short and state
  facts (fragment placement, pricing basis); no plan or history
  prose in code — the only cycle mentions are the two tripwire
  annotations and the test-file header, matching existing tree
  convention. No `unwrap`/`expect`, no `.into()`, no glob imports
  in the new `src/` code (grep clean outside tests and a doctest).
  `DESIGN.md` and `STYLE.md` untouched, correctly left to the
  applicator. `src/lib.rs` stale "no style object" line corrected
  (diff vs `git show HEAD:src/lib.rs`).

## Evidence (reviewer-run)

- `sh check.sh`: **all gates green** (fmt, tests, clippy
  `-D warnings`, docs). `cargo test --locked`: 35 unit, integration
  1+6+1+4+4+10 (styles: 4), doctests 5 passed + 3 (compile-fail
  expectations) passed, 0 failed anywhere.
- Styles part probe (python, XML parse + zlib CRC): len 2235, CRC
  `76B2525A`, counts and palette order as above; HEAD recomputed
  708/`24A3B074`.
- HEAD byte probe (`/tmp` crate against the working tree and a
  `git archive HEAD` extraction, same 2000-row ints/floats recipe):
  current `oracle_len=150755 oracle_crc=44F917A2 archive_len=155366`;
  HEAD `oracle_len=150755 oracle_crc=44F917A2 archive_len=153839`.
  Sheet oracle byte-identical; archive delta exactly 1527.
- Macro refusal probe: five spellings above, each
  `error: column style grammar: expected \`bold\`, one colour …`.
  Non-identifier `[123]`: `error: no rules expected \`123\`` (M1).
- `cargo run --release --example styled`: writes a 5419-byte file.
- Bench spot check (criterion `--quick`, single reads, this
  session): `build_styled_mixed_200000` 26.49 ms vs twin
  `build_mixed_200000` 23.18 ms — same gap order as the recorded
  Post-018 pairs (25.58/25.83 vs 23.02/23.19 ms); no new speed claim
  is made from these quick reads. Bench binary builds clean
  (`cargo bench --bench fill_write --no-run`).
- Tree identity: before hashes (header of this file) re-verified
  after all runs; only `specs/018-styles/REVIEW.md` (this file) is
  new. Not committed.

## Severity summary

| Severity | Count | Items |
|---|---|---|
| Blocker | 0 | — |
| Major | 0 | — |
| Minor | 2 | M1 macro wording for non-identifier tokens; M2 sampled xf assertions |
| Observation | 2 | Dual index spelling; cap over-reserve on bold-only (by design) |
