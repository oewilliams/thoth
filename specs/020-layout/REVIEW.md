# Cycle 020 — reviewer report

Base: HEAD `a0fddc9`. Scope reviewed: the uncommitted 020 diff
(`src/contract.rs`, `src/emit.rs`, `src/xlsx.rs`, `TESTING.md`
Post-020 entry) against `specs/020-layout/SPEC.md`, `STYLE.md`,
`DESIGN.md`, `TESTING.md`. The pre-existing uncommitted
`RUNOFF.md` entry was excluded from findings, as instructed.
Verdict: **do not commit as-is.** One blocker and one major
finding must be fixed first; both sit inside the shipped E4/E1
code. Everything else checked either passes or is listed as
minor. No commit was made.

## Reviewer tree-integrity statement

Before any probe ran, and again after all probes finished, the
source tree (excluding this `REVIEW.md`) was hashed
(`shasum -a 256`) and `git status --porcelain` was captured.
The two captures are identical: `src/contract.rs`
`dd236e74…`, `src/emit.rs` `6e30faa7…`, `src/xlsx.rs`
`f98ef315…`, `src/zip.rs` `636ac5a3…`, `TESTING.md`
`73997e45…`, plus unchanged `STYLE.md`/`DESIGN.md`/`SPEC.md`,
and the same status lines (`M RUNOFF.md`, `M TESTING.md`,
`M src/contract.rs`, `M src/emit.rs`, `M src/xlsx.rs`,
untracked `.DS_Store`, `external/`, `specs/020-layout/`).
All probe code lived in `/tmp/review020` and
`/tmp/thoth-head-review`; the only file this review created in
the tree is this `REVIEW.md`. The tree is byte-identical
before and after the review work, excluding `REVIEW.md`.

## Blocker

### B1. `escape_into` panics on valid multi-byte text (E4 regression)

`escape_into` in [src/emit.rs](/Users/oliverwilliams/Documents/thoth/src/emit.rs:87)
skips clean 8-byte words with `i += 8` and later slices
`&text[start..i]` (line 103; also `text[i..]` at line 114).
The skip only tests `swar_has_escapable`, so a word holding
multi-byte UTF-8 bytes and no escapable is "clean" and the
+8 step can land inside a multi-byte char. When a later word
holds an escapable, the flush slices mid-char and Rust
panics: `byte index 16 is not a char boundary`. The doc claim
at [src/emit.rs](/Users/oliverwilliams/Documents/thoth/src/emit.rs:86)
("slice points sit on them or on the text ends") is false for
this case. The byte sink `escape_into_bytes` is unaffected
(it slices bytes, never `str`).

Evidence (scratch crate `/tmp/review020`, shipped code called
directly; char-loop oracle as expected output):

- Targeted case `"abcdefg£hijklm&"` (£ straddles the first
  word boundary) panics; `aaaaa😀&tail` and siblings panic.
- Systematic sweep (16 prefix lengths × 4 multi-byte chars ×
  16 mid lengths × escapable-or-none, 6,144 strings): **484
  panics, 0 wrong outputs**. Whenever it does not panic the
  spelling is correct — the defect is purely the slice point.
- Reachable from production entry points, not just the helper:
  `build_workbook` with a sheet named `abcdefg£hijklm&`
  panicked in my probe (sheet names escape through
  `escape_into` in `src/workbook_parts.rs`); `sheet_xml` over
  string cells holding the straddle texts panicked, while
  `build_workbook` (byte sink) over the same cells succeeded
  and was byte-identical to a `git archive` HEAD build. So the
  permanent oracle and the arena now disagree by crashing —
  R1 cannot even be produced for this input class, and a
  library panics on data input, against STYLE rule 8 (panic
  is programmer error; data conditions get `Err`).

Why the shipped tests miss it: `escape_spelling_is_pinned`
([src/emit.rs](/Users/oliverwilliams/Documents/thoth/src/emit.rs:712))
places its only multi-byte char (£) inside an already-dirty
word, where the per-char walk handles it. No case puts a
multi-byte char across a clean-word boundary with an
escapable in a following word. Required fix shape (reviewer
does not prescribe the spelling): keep `str` slice points on
char boundaries — e.g. only take the SWAR skip when the word
is plain ASCII, as the boundary scan already does — and add
pinned cases for 2-, 3- and 4-byte chars straddling word
boundaries at every offset, with and without a later
escapable.

## Major

### M1. The E1 witness is not exact: `swar_count_byte` over-counts

SPEC E1 and the record require the *exact* escaped length
([SPEC.md](/Users/oliverwilliams/Documents/thoth/specs/020-layout/SPEC.md:49),
[src/contract.rs](/Users/oliverwilliams/Documents/thoth/src/contract.rs:430),
[TESTING.md](/Users/oliverwilliams/Documents/thoth/TESTING.md:1399)).
The count in
[swar_count_byte](/Users/oliverwilliams/Documents/thoth/src/emit.rs:41)
uses `(x − ONES) & !x & HIGHS` per lane. That mask is sound
for *existence* (a borrow only starts at a true zero lane, so
`swar_has_byte` never invents a hit), but lane counting is
not: a borrow out of a zero lane also flags a following lane
whose `x` byte is `0x01` (i.e. text byte `target ^ 1`). So an
escapable immediately followed (in byte order) by its
`^1` neighbour — `&'` or `'&`, `<` followed by `=`, `>`
followed by `?`, `"` followed by `#` — is counted twice for
the first byte's target.

Evidence against the shipped code, end to end through
`validate_template_witnessed` versus a per-char oracle:
`&'&'&'&'` witnesses 75 against the true 44; `'&'&'&'&`
witnesses 76 against 44; `&<>"'&<>` witnesses 43 against 38;
`aaa&'aaa` witnesses 21 against 17. Four of my ten pattern
words were inexact. A direct sweep of the copied formula over
2,000,000 random words × 7 target bytes found 1,497
mismatched counts, all over-counts. Consequences, stated
plainly: output bytes are **not** affected (my whole-archive
checks below are identical to HEAD), and the error only ever
inflates the reservation, never shrinks it below the true
need, so no build can fail from it. But the "exact" claim in
the SPEC deliverable, the contract doc, the test name
`witnessed_escaped_total_is_exact`
([src/contract.rs](/Users/oliverwilliams/Documents/thoth/src/contract.rs:869))
and the TESTING record is false as shipped, and the retained
capacity figures (1.044x / 1.026x) were measured on recipes
whose words may include this inflation. Fix the count
(exact per-lane counting costs the extra mask steps) or drop
the word "exact" everywhere and re-measure; the SPEC's own
text favours fixing it. Add pattern cases above to the
witness test.

## Minor

- **Test gap that hid B1.** Beyond the fix, the pinned escape
  test should carry straddle cases permanently; today the two
  sinks "share the scan and cannot pin each other" is guarded
  only for ASCII placements. ([src/emit.rs](/Users/oliverwilliams/Documents/thoth/src/emit.rs:712))
- **Doc wording.** [src/contract.rs](/Users/oliverwilliams/Documents/thoth/src/contract.rs:433)
  says on refusal "the total is partial". `scan_text_cells`
  ([src/contract.rs](/Users/oliverwilliams/Documents/thoth/src/contract.rs:507))
  in fact accumulates over every cell and the total is then
  discarded. Harmless, but the sentence misdescribes the code.
- **STYLE tripwire annotation is stale.**
  [src/emit.rs](/Users/oliverwilliams/Documents/thoth/src/emit.rs:5)
  and `src/contract.rs:6` carry the cycle-018 annotation for
  passing 700 lines; 020 grew both files further
  (749→925 and 806→950 lines by `wc -l` against HEAD). The
  breach is pre-existing and annotated, so this is a nit, but
  the annotation should own the new size. All new functions
  are under the 100-line tripwire (largest: `scan_one_text`,
  73 lines).

## What was independently verified (reviewer's own runs)

- **Gate.** `sh check.sh` re-run in this tree: green (fmt, 40
  unit + integration tests, clippy `-D warnings`, docs).
- **R1 byte identity (spot, independent).** Against a
  `git archive` HEAD extraction built as a second crate:
  escapable-heavy 20k strings, 50k ints, 2,000 long strings
  and a multi-byte string set through the byte sink — all
  whole-archive identical to HEAD. The producer's wider list
  (bucket, liability, ByAsset-like, three-sheet, styled) was
  not re-run and is relied on as recorded.
- **Assembly claims (E4).** `--emit asm` on the release lib:
  `escape_into_bytes` loads one `ldr x` per 8-byte word,
  runs five `eor`/`sub` lane tests, has no `bl` and no NEON
  (`q`/`.16b`) on the clean path; `ldrb` appears only in the
  dirty-word walk. Scalar SWAR as claimed, not vectorised.
  `src/zip.rs` has an empty diff, so the `crc32x` spelling
  is unchanged by construction.
- **Shipped candidate timing (E4, interleaved, independent).**
  Within one scratch binary alternating shipped-tree and
  HEAD-tree `build_workbook`, 7 rounds, long-string recipe
  (2,000 × ~4,200 B): shipped median 3.35 ms against HEAD
  23.67 ms, ratio **0.142** — agreeing with the recorded
  0.134–0.140 (different row count, same shape family).
  Identity was asserted before timing.
- **E1 capacity direction.** On my escapable-heavy recipe the
  returned `Vec` capacity/length moved from 1.182 (HEAD) to
  1.028 (shipped). The recorded 1.575→1.044 / 1.550→1.026
  recipes could not be reproduced exactly (TESTING truncates
  the cell text with "…"); direction and rough size agree,
  subject to M1's inflation caveat. Peak-RSS parity is taken
  from the record, not re-measured.
- **Rejected candidates leave no code.** The diff touches
  only `src/contract.rs`, `src/emit.rs`, `src/xlsx.rs`,
  `TESTING.md`. No staging buffer, touch-ahead or fold
  threshold text exists in `src/` (searched); E5's sweep
  threshold and E2's 64-column threshold stayed in scratch.
- **No payload classification past the boundary.** The diff
  branches only on bytes for spelling (escapables) and, at
  the boundary, on validity — never on what a numeric value
  means; expected widths 10 (`I64`) / 12 (`F64`) remain in
  `sheet_hint_inner`
  ([src/contract.rs](/Users/oliverwilliams/Documents/thoth/src/contract.rs:753)).
  The numeric-witness rejection is consistent with this rule.
- **Tunables.** No new numeric tunable ships: the only new
  constants are the SWAR masks, fixed by the `u64` word
  width. Sweep-winner recording (STYLE rule 5) is therefore
  satisfied vacuously for shipped code; the E1 witness rides
  the existing boundary pass (single traversal in
  `scan_text_cells`), adding no pass.
- **STYLE mechanics.** No `unsafe`, no `#[allow]`, no glob
  import and no bare `.into()` in the diff; the only new
  `expect` sits in a test, which the Rust chapter permits.

## Relied on, not re-run

The E2/E3/E5 scratch numbers, the R3 narrow-recipe spreads,
the bucket/liability timings, peak RSS, and the producer's
full R1 recipe list are recorded consistently in TESTING
Post-020 with machine, profile and commit stated, and nothing
I re-ran contradicts them. They are accepted as the
producer's reads, not re-verified here.

## Required before commit

1. Fix B1 and pin the straddle cases in the escape test.
2. Fix M1's counting or retract "exact" in SPEC-facing docs,
   the contract doc, the test name and TESTING, with
   re-measured capacities if
...[truncated 307 chars]