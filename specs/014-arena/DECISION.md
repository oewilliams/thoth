# Cycle 014 — arena: applicator decision

Applicator, independent of producer and reviewer. Everything left
uncommitted. Machine: Darwin 27.0.0 arm64, Apple M4, rustc 1.93.1,
bench/release profile, commit `2a51dc9` plus the uncommitted 014
work. This session ran elevated (1-minute load 1.7–1.9, rising
across the spells); where that matters it is named.

## Required change 2 (STYLE §1, `build_workbook` length): APPLIED

`build_workbook` was 115 lines in a 697-line file, over the
100-line function limit with no annotation. Split, as the reviewer
suggested: the emission-and-central tail (small parts, sheets,
central directory, end record) is now the private helper
`emit_parts` in `src/xlsx.rs`, called once from `build_workbook`;
all cast-cover comments moved with the code they cover, unchanged.
After the split: `build_workbook` 78 lines, `emit_parts` 46 lines,
file exactly 700 lines (not over). Four neighbouring doc comments
were tightened by a few lines to hold the file limit; no claim was
dropped (the `archive_fits` cast list is untouched). No behaviour
change: byte-identity below is the proof.

## Required change 1 (R4 null parity): NOT APPLIED as a code change — disagreement recorded, bar referred to the human

The reviewer's own alternative is taken: parity was attempted,
measured unreachable with the safe spellings tried, and is recorded
here with numbers for a human re-judgement rather than silently
inherited. Two disagreements with the review's framing are recorded
first, before any code conclusion:

1. The deficit this applicator could reproduce is smaller and less
   stable than "consistent 0.6–3.8%" suggests once binary layout
   changes: the emitter probe (the producer's 9×15 rotating-order
   method, temporary `__probe_sheet_bytes` wrapper plus a scratch
   example, both removed before delivery) read, per spell on the
   null recipe at 200k — oracle / bytes: 4.029/4.144ms (+2.9%),
   4.027/4.079 (+1.3%), 3.951/4.081 (+3.3%) in the quiet opening
   spells, and +6–8% in later spells after unrelated probe code
   changed the binary layout. The sign is consistent; the magnitude
   is layout-sensitive at the same order as the effect, which the
   review's beyond-noise reading does not separate.
2. The probe as rebuilt includes the per-row CRC fold (R3's locked
   spelling) on the byte side and none on the oracle side. A
   no-CRC isolation variant (same probe binary, indicative only)
   read null at +0.3% in its best spell and +1.6–4.1% in others.
   Part of the measured gap is therefore the checksum the spec
   locks in, not emitter appends; at build level the arena already
   beats the oracle's emission alone (Post-014 criterion: build
   null 4.03–4.11ms vs emission 4.21–4.23ms).

Attempts made and reverted on their numbers (STYLE §1: a faster
formulation enters with a measurement, never a belief):

- Blank cell assembled in a 24-byte stack buffer, one arena append
  instead of four: null bytes 5.00–5.10ms vs oracle 3.94–4.01ms
  (+25%), ints also regressed. The dynamic-index stack copies cost
  more than the appends they save. Reverted.
- Branch-first blank/value split in `i64_cell_bytes` /
  `f64_cell_bytes` (same appends, null branch taken before the
  open): null +23%, ints +5–8% — duplicated opens bloat the inlined
  loop. Reverted.
- The producer's precomputed-prefix attempt (6–17% slower) was not
  retried, per the review.

`src/emit.rs` therefore ships byte-identical to the reviewed tree
(verified by diff against the pre-applicator snapshot): no emitted
byte changes, and no unmeasured spelling is carried. Recommendation
for the human's re-judgement: judge R4 at build level, where the
combined shape is what ships, or keep the emitter-level bar and
accept the recorded ~1–3% null deficit as the price of the locked
per-row fold. Either way the deficit is now named, not inherited.

## DESIGN.md (applicator's step, per spec scoped files): APPLIED

Floor now records the reserved 30-byte header patched at the known
length, the `(name, crc, len, offset)` entry records, the per-row
checksum fold, and the archive builder as the zip layer's single
internal spelling. Templates now records the one-reservation /
block-growth cases exactly as locked, the worst-case bound as proof
rather than reservation, the recorded-retention rule, and that no
current caller declares an unknown dimension (that spelling arrives
with the template types). No other DESIGN wording changed.

## Final verification (applicator's own runs, final tree)

- `cargo fmt`: applied; `cargo fmt --check` clean afterwards.
- `sh check.sh`: green, exit 0 — format check, 33 unit tests,
  round-trip, multi-sheet, four sealed tests, clippy `-D warnings`,
  docs ("all gates green").
- Byte-identity: `build_workbook` dumps from the final tree at
  200k rows (ints, floats, null, mixed, three-sheet) compare
  byte-identical (`cmp`) to the HEAD dumps; lengths
  10470009 / 10870010 / 10470071 / 16914521 / 21338118, retained
  capacities unchanged (null 20203485/10470071 = 1.93×, recorded
  for the successor's hint decision as R2 requires).
- Build-time sanity (elevated session, medians of 15 builds):
  ints 4.19–4.38ms, floats 9.74–10.20ms, null 4.29–4.46ms against
  Post-012 build pairs 4.80 / 9.87 / 5.20–5.23ms — uniform session
  elevation, slowest on the untouched float path, no
  split-attributable regression read. Also recorded in TESTING.md.
- Scoped files only: `src/xlsx.rs` (split plus doc tightening),
  `DESIGN.md`, `TESTING.md` (applicator re-check), and this file.
  `src/emit.rs`, `src/zip.rs`, `tests/`, `examples/`, and manifests
  are untouched by the applicator; all temporary probe code was
  removed (tree carries no `__probe` spelling).

## Outcome

R1, R2, R3, R5, R6 stand as reviewed. Required change 2 is applied.
Required change 1 is referred to the human with the numbers above:
the emitter-level null bar is not met and no measured safe spelling
met it this cycle; the build-level shape beats the oracle on every
recipe measured. The cycle is shippable on the human's re-judgement
of that one bar.

## Human re-judgement (closing the referred item)

The human is not worried about the null recipe: the ~1-3%
emitter-level null deficit recorded above is accepted as the
price of the locked per-row fold, and R4 is judged at build
level, where the arena beats the oracle's build on every recipe
including nulls. Cycle 014 ships as applicated.
