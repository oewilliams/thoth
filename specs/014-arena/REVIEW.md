# Cycle 014 — arena: review

Reviewer, independent of the producer. Tree snapshot before review
(`/tmp/review014_before_hashes.txt`, 64 files outside `.git`/`target`,
plus `git diff`): six uncommitted files — `src/emit.rs`, `src/zip.rs`,
`src/xlsx.rs`, `src/lib.rs`, `benches/fill_write.rs`, `TESTING.md`.
`tests/`, `examples/`, manifests untouched. This reviewer's only write
is this file; the closing snapshot below proves it.

Machine: Darwin 27.0.0 arm64, Apple M4, rustc 1.93.1, bench/release
profile (`overflow-checks = true`, `codegen-units = 1`), HEAD `2a51dc9`
plus the uncommitted 014 work. Reviewer probes lived in `/tmp/probe014`
(HEAD extracted to `/tmp/thoth-head014`); nothing probe-shaped remains
in the tree.

## Verdicts

### R1 — Arena builder: PASS

Inspected: `build_workbook` emits into the one returned `Vec`
([src/xlsx.rs](/Users/oliverwilliams/Documents/thoth/src/xlsx.rs:530)):
`begin_entry` reserves the 30-byte header plus name
([src/zip.rs](/Users/oliverwilliams/Documents/thoth/src/zip.rs:187)),
sheets and small parts emit directly after it
([src/xlsx.rs](/Users/oliverwilliams/Documents/thoth/src/xlsx.rs:495)),
`patch_local_header` writes CRC and equal stored sizes at +14/+18/+22
([src/zip.rs](/Users/oliverwilliams/Documents/thoth/src/zip.rs:209)),
entries are `Vec<(String, u32, usize, usize)>` records owning no data
([src/xlsx.rs](/Users/oliverwilliams/Documents/thoth/src/xlsx.rs:601)),
central directory and end record append last
([src/xlsx.rs](/Users/oliverwilliams/Documents/thoth/src/xlsx.rs:630)).
`sheet_xml` remains the standalone oracle spelling
([src/xlsx.rs](/Users/oliverwilliams/Documents/thoth/src/xlsx.rs:515)).

Byte-identity, re-run by this reviewer (HEAD vs current `build_xlsx` /
`build_workbook` at 200,000 rows, full `Vec` equality in `/tmp/probe014`):
ints (len 10470009), floats, null, mixed, and a three-sheet build are
byte-identical on every recipe; verbatim-embed holds. My recipes differ
slightly from the bench recipes, so lengths differ from TESTING.md's
where values differ; identity, not length, is the claim and it holds.
Gate tests green unmodified in `tests/`: round-trip with verbatim-embed
([tests/roundtrip.rs](/Users/oliverwilliams/Documents/thoth/tests/roundtrip.rs:17)),
three-sheet round-trip with per-sheet verbatim-embed
([tests/multi_sheet.rs](/Users/oliverwilliams/Documents/thoth/tests/multi_sheet.rs:19)),
cat-identity/static pins
([src/workbook_parts.rs](/Users/oliverwilliams/Documents/thoth/src/workbook_parts.rs:231)).

One wording note, not a failure: the golden lock's construction was
re-spelled through the arena primitives
([src/zip.rs](/Users/oliverwilliams/Documents/thoth/src/zip.rs:317))
because retiring `ZipWriter` (spec-mandated) made the old construction
unwritable. Its expected 207 bytes are unchanged and the test passes;
the byte contract, not the construction, is what R1 pins.

### R2 — Reservation follows the template declaration: PASS, two observations

Inspected: reservation is one `Vec::with_capacity(stored_archive_len(&hint_sizes))`
([src/xlsx.rs](/Users/oliverwilliams/Documents/thoth/src/xlsx.rs:599))
from `sheet_hint` (typical widths I64 10 / F64 12,
[src/xlsx.rs](/Users/oliverwilliams/Documents/thoth/src/xlsx.rs:331));
`sheet_cap` survives only as the ceiling projection and the
`sheet_cap_holds` proof
([src/xlsx.rs](/Users/oliverwilliams/Documents/thoth/src/xlsx.rs:305)).
Correctness never depends on the hint: emission appends and the buffer
grows if the hint under-estimates. Reviewer-measured retained
capacity/length at 200k: ints 12203418/10470009 = 1.17×, floats 1.17×,
null 20203485/10470071 = 1.93× — matching the producer's recorded caps
exactly where shapes match. The null 1.93× retention is recorded in
TESTING.md (Post-014) for the successor's decision, as R2 requires; no
silent several-times retention (1.93× is under 2× and is named).

Observations (not blocking): (a) no unknown-dimension caller exists in
the current API — `validate_columns` fixes `nrows` before emission, so
the known-shape case is the only reachable one and block growth is
correctly deferred to the template types, as the `sheet_hint` doc
states; (b) derived from the code (not measured): for sheets of widest
floats (`1e300` spells 301 digits vs the hint's 12) the hint
under-reserves and the arena grows — correctness is unaffected, and
the measured recipes never grow, but a future widest-float bench would
show reallocations the design lock's "one reservation, no realloc"
wording does not admit. The successor's hint decision (already owed
for nulls) should cover this case explicitly.

### R3 — CRC folded per row: PASS

Inspected: chunked spelling `CRC_INIT` / `crc32_update` / `crc32_finish`
beside one-shot `crc32`
([src/zip.rs](/Users/oliverwilliams/Documents/thoth/src/zip.rs:90));
sheets fold once per emitted row over the arena range — header fold,
one fold per data row, epilogue fold
([src/xlsx.rs](/Users/oliverwilliams/Documents/thoth/src/xlsx.rs:452)).
`crc_update_any_split_matches_one_shot` pins every split of a 1024-byte
probe, empty chunks, and byte-at-a-time against one-shot, green in my
gate run.

Whole-sheet `crc32` on the build path — reviewer grep of `src/` and
`benches/`, every surviving one-shot call site: exactly one on the
build path, `emit_small_part`'s CRC of its just-emitted small-part
range ([src/xlsx.rs](/Users/oliverwilliams/Documents/thoth/src/xlsx.rs:504)),
which R3 explicitly allows. All other hits are test-only: the pin
assertions in `src/workbook_parts.rs` tests, `crc_is_ieee` /
split-test / golden construction in `src/zip.rs` tests, and the
definition itself. No whole-sheet call survives. PASS.

### R4 — Byte sink, tuned to parity: NOT MET on nulls (required change 1)

Byte correctness passes: `byte_sink_matches_string_oracle`
([src/emit.rs](/Users/oliverwilliams/Documents/thoth/src/emit.rs:336))
pins the byte emitters against the `String` oracle over the i64 sweep,
f64 specials plus 2,000 quarter-steps, and escaped headers, green in my
run; the byte-identity evidence under R1 covers the composed path.
Digits come from `DEC_PAIRS`, fragments are byte constants, the f64
spelling writes through a `fmt::Write` adapter
([src/emit.rs](/Users/oliverwilliams/Documents/thoth/src/emit.rs:159)).

Emission parity does not fully pass. The producer's own recorded
interleaved probe (TESTING.md, Post-014) reads ints within noise,
floats 2–5% faster, and nulls a consistent ~0.6–3.8% slower — the
prototype's tuned deficit, carried over. R4's acceptance is explicit:
no regression beyond noise on ints or nulls on agreeing reads, and
"the prototype's known deficit is this cycle's work, not its
inheritance." A consistent directional deficit across spells is
beyond noise by the project's own spell-agreement reading. My own
probe could not isolate the emitter (build includes CRC and zip;
floats build ≈ oracle emission at 1.003× despite that extra work,
corroborating the float claim), and re-running the producer's direct
emitter probe would require re-adding the removed temporary wrapper
to `src/`, which this role may not do — so the null verdict rests on
the producer's recorded numbers, quoted above, which the producer
transparently flagged for this judgement. Judgement: the bar is not
met on nulls.

Required change 1 (applicator): make the blank/null byte path reach
noise-parity with the `sheet_xml` oracle on the null recipe on
agreeing interleaved reads (the producer's 9×15 rotating-order probe
or equivalent), without changing any emitted byte. The rejected
precomputed-prefix attempt (6–17% slower, reverted) need not be
retried as-is. If parity proves unreachable, record that with numbers
in DECISION.md and let the human re-judge the bar — do not silently
inherit the deficit again.

### R5 — Zip32 ceiling re-derived: PASS

Reviewer re-derived the ceiling cast for cast. `archive_fits`'s body
is unchanged from HEAD; its doc now re-lists every category
([src/xlsx.rs](/Users/oliverwilliams/Documents/thoth/src/xlsx.rs:248)).
Every narrowing cast on the arena path and its cover:

| Cast | Cover in `archive_fits` |
|---|---|
| small-part `data.len() as u32` (xlsx:507) | actual lengths in `sizes`; `data_len <= DATA_MAX` |
| sheet `len as u32` (xlsx:625) | `sheet_cap` projection checked; actual ≤ cap (`sheet_cap_holds`) |
| `central_start as u32` (xlsx:630) | local-section total `datas + names + 30n <= TOTAL_MAX`; projection ≥ actual |
| central `len`/`offset as u32` (xlsx:635) | entry lengths as above; offsets precede central start |
| `central_len as u32` (xlsx:641) | central-section total `names + 46n <= TOTAL_MAX`, exact (names actual) |
| `name.len() as u16` (zip:200, zip:237) | `name_len <= NAME_MAX` for every entry incl. sheet names |
| `count as u16` (zip:260) | `sizes.len() <= COUNT_MAX` |

Small parts enter at actual length; sheets at the conservative
`sheet_cap` projection, named with its reason in the doc — exactly
R5's rule. All six `archive_fits` ceiling tests, `sheet_cap_holds`,
and the 011 many-sheet count test are green in my gate run. No cast
on the path lacks a cover; the `central_start as usize` widening at
xlsx:641 is lossless. PASS.

### R6 — Measured acceptance, reframed: PASS (reported, not gated)

TESTING.md's Post-014 entry re-takes the 012 phase recipes and records
byte-identity, the RSS probe, agreeing criterion pairs against
Post-012, interleaved A/B, retained-capacity table, discarded reads,
and machine/profile/commit. Reviewer re-derivation (single-binary
interleaved medians, 5 rounds × 15 builds at 200k, `/tmp/probe014`):
build HEAD → current ints −13.8%, floats −8.3%, null −14.9% — no
recipe regresses; all three sit in or above the prototype's range
(ints ~8.5–11%, floats ~7.7–9%, nulls ~2–3%), with smaller magnitudes
than the producer's cross-binary reads (−21/−16/−19%), which is
session variance, not contradiction. Peak memory re-derived on my
three-sheet recipe (archive 27,382,637 bytes, held alive, 3 runs each,
`/usr/bin/time -l`): HEAD 60,063,744 bytes every run, current
32,620,544–32,636,928 — the double residency is gone; the new peak
tracks the one arena's capacity (32,405,116) plus ingress. This
corroborates the producer's spec-recipe figures (47,923,200 →
26,558,464). The criterion float/null emission deltas against
Post-012 are cross-session drift on untouched code, correctly
reported rather than gated. PASS.

## Style finding (required change 2)

`build_workbook` grew from 85 lines at HEAD to 115 lines
([src/xlsx.rs](/Users/oliverwilliams/Documents/thoth/src/xlsx.rs:530)),
over STYLE §1's 100-line function limit, with no written annotation
on the item arguing the exception; the file sits at 697 of 700 lines.
Required change 2 (applicator): split the function (the pre-pass /
reservation / emission / central phases separate naturally) or add
the written annotation STYLE requires. No other style deviation
found: imports itemized, conversions spelled, no `unwrap`/`expect`
added in `src/`, `ZipWriter` fully retired (zero hits in `src/`,
`benches/`, `tests/`), `zip` now `pub(crate)`
([src/lib.rs](/Users/oliverwilliams/Documents/thoth/src/lib.rs:13)),
public signatures `sheet_xml` / `build_workbook` / `build_xlsx` /
`write_workbook_file` / `write_xlsx_file` unchanged, no new
dependencies. DESIGN.md untouched — correctly the applicator's step.

## Gate (reviewer's own run)

`sh check.sh` in the reviewed tree: format check, 33 unit tests,
round-trip, multi-sheet, four sealed tests, clippy `-D warnings`,
docs — "all gates green", exit 0.

## Tree identity

Before: 64-file SHA-256 manifest and `git status`/`git diff` snapshot
in `/tmp/review014_before_*`. After writing this file, re-hashed:
every pre-existing file byte-identical, `git status` differs only by
the new `specs/014-arena/REVIEW.md`. The reviewer changed no source.

## Required changes

1. R4 null parity (above): blank-cell byte emission to noise-parity
   with the oracle on agreeing interleaved reads, bytes unchanged —
   or a numbered, human-judged re-bar in DECISION.md.
2. STYLE §1: `build_workbook` back under 100 lines, or the required
   written annotation on the function.

No other changes required. R1, R2, R3, R5, R6 pass as built.
