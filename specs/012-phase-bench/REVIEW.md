# Cycle 012 — phase bench: review

Reviewer, independent of the producer. Scope read: `specs/012-phase-bench/SPEC.md`,
`STYLE.md`, `DESIGN.md`, `TESTING.md` (all binding), the `phase` group in
`benches/fill_write.rs`, and the Post-012 entry in `TESTING.md`. No source was
edited; the only file this review creates is this one, left uncommitted.

Tree discipline: opening snapshot `git status --porcelain` showed the
uncommitted 011+012 stack on `fe0e896` and `git diff | sha256sum` =
`09a9678045314d8f4b9a914fc452e704cdcf1521a079a31036a55bdde31444f0`. After all
checks below (gate, bench re-run, `/tmp` probe), the same command returned the
identical hash and status; `git diff --stat src/` is unchanged. Outside this
file the tree is byte-identical to the opening snapshot.

**Overall verdict: accept.** Both 013 fusions are correctly recorded **no-go**;
the next target is the emitter. Verdicts per requirement follow.

## R1 — Phase recipes beside the existing groups, same scales: PASS

The producer's one new group is `phase` (`benches/fill_write.rs:197-268`),
registered at `benches/fill_write.rs:270`. Every timed closure, listed in full:

- Single-sheet emission (`benches/fill_write.rs:206-208`, `:218-220`,
  `:230-232`): exactly
  `b.iter(|| std::hint::black_box(thoth::xlsx::sheet_xml(&cols).unwrap()))`.
  One statement, result `black_box`ed, no file touched.
- Single-sheet build (`benches/fill_write.rs:209-211`, `:221-223`,
  `:233-235`): exactly
  `b.iter(|| std::hint::black_box(thoth::xlsx::build_workbook(&sheets).unwrap()))`.
  No file touched.
- Three-sheet emission (`benches/fill_write.rs:257-263`): one
  `black_box(sheet_xml(sheet.columns).unwrap())` per sheet in a loop over the
  prebuilt sheet set — `sheet_xml` per sheet, summed, as R1(a) requires.
- Three-sheet build (`benches/fill_write.rs:264-266`): exactly
  `black_box(build_workbook(&sheets).unwrap())`.

Setup stays out of the clock: `writer_columns()` and the `Sheet` vectors are
built before each `bench_function` (`benches/fill_write.rs:200-205`,
`:212-217`, `:224-229`, `:237-256`) and the closures use `b.iter`, not
`iter_with_setup`, so nothing per-iteration runs except the phase call. The
`phase` function contains no `remove_file`, `write_path`, or any `fs::` call
(grep over `benches/fill_write.rs:197-268` is empty), so no emit/build recipe
touches a file. The emission allocation is inside the clock, exactly as the
library performs it (`sheet_xml` returns a fresh `Vec<u8>`,
`src/xlsx.rs:498`; `build_workbook` at `src/xlsx.rs:568`), and the header
comment names this (`benches/fill_write.rs:188-196`). `black_box` use is
correct: it consumes each result by value, so neither the allocation nor the
formatting can be elided, and no dead result accumulates across iterations.

Recipes and scales match R1: `tall_ints`, `tall_floats`, `tall_null` (the
`floor` `all_null` recipe, `benches/fill_write.rs:59-75`) at 20_000 and
200_000 (`benches/fill_write.rs:199`), plus the 011 three-sheet example at
20k ints + 20k floats + header-only (`benches/fill_write.rs:237-256`). The
diff against HEAD appends only `multi_sheet` (011's group) and `phase`; the
`fill_write`, `floor`, `cells`, and `crc` function bodies are untouched, and
end-to-end is never re-implemented in `phase` — the Post-012 table re-uses
the existing groups' numbers. `cargo bench` compiles clean (the binary below
ran; `sh check.sh` clippy `--all-targets` also covers this file).

## R2 — Phase table with shares as ranges: PASS (two minor observations)

The Post-012 entry (`TESTING.md:675-879`) states machine, profile, commit,
binary, and box state (`TESTING.md:675-708`), a claimed-pairs table with
means, CIs, and severe-outlier shares (`TESTING.md:713-731`), and the phase
table with every share as a range (`TESTING.md:738-746`). I re-derived the
arithmetic independently from the quoted reads and byte counts
(`TESTING.md:698-703`); every figure below matches the entry to its printed
rounding:

- Subtractions. Floats 200k: build − emission = 9.8697−8.6158 to
  9.8778−8.6057 = 1.2539–1.2721 ms; write − build = 1.8392–1.8893 ms. Ints
  200k: 1.1553–1.1682 ms and 1.8406–1.9277 ms. Null 200k: 1.1435–1.1730 ms
  and 1.8898–1.9147 ms. Three-sheet: 0.2309–0.2334 ms and 0.3033–0.3038 ms.
  All 20k subtractions likewise reproduce exactly.
- Shares. Floats 200k (the claimed denominator): emission 73.2–73.5%,
  CRC + assembly 10.7–10.9%, file + seal 15.6–16.1%. Ints 200k on the
  isolated (†) reads: 54.2–54.8% / 17.2–17.5% / 27.3–29.0%. Null 200k (‡):
  57.0% / 16.1–16.5% / 26.5–26.9%. Three-sheet (‡): 68.2–68.3% / 13.7–13.8% /
  18.0%. No share is a single-point percentage where a range exists; the
  †/‡ denominators are explicitly demoted to corroboration
  (`TESTING.md:748-765`), which is the honest handling of the
  cross-process subtraction trap named in the spec.
- CRC anchor. The claimed `crc32_1m` pair (0.0959–0.0965 ms/MiB =
  0.0915–0.0920 ns/B, ~10.9 GB/s) applied to each sheet byte count predicts
  0.960 ms (ints 200k), 0.997 ms (floats 200k), 0.960 ms (null 200k),
  0.185 ms (three-sheet) inside build − emission — 78–85% of every measured
  segment, with a small positive copy remainder in each case. No subtraction
  disagrees with its anchor beyond the CIs, so the entry correctly invokes
  no anchor-override (`TESTING.md:767-782`).
- `tall_null` floor anchor. Blank-cell emission (4.0559–4.0605 ms, two
  columns) exceeds the whole integer emission (3.6486–3.6492 ms, one
  column), so the float premium over skeleton-and-tags is at least
  4.545–4.560 ms of the 8.6057–8.6158 ms float emission. The entry states
  ~4.5 ms as a floor with the two-vs-one-column caveat
  (`TESTING.md:782-791`); the bound is generous in the safe direction.
- Protocol. Spot-checked claimed pairs satisfy the spell-agreement rule:
  means within 0.02–1.5% of each other, CI half-widths 0.05–1.9% (under
  5%), severe outliers ≤ 10% throughout. Emission rates re-derive from
  sheet bytes (ints 200k 2868 MB/s, floats 1261–1263 MB/s, null
  2578–2581 MB/s) and end-to-end rates from archive bytes (floats 200k
  924–928 MB/s), matching the table.

Minor observations, neither blocking: (1) the copy-remainder ranges
(`TESTING.md:776-779`) subtract the anchor's mid-point prediction rather
than its full range, so each remainder is ~3 µs narrower per side than a
full-range subtraction (ints: entry 0.1950–0.2079 ms vs full-range
0.1920–0.2109 ms); shares move by ≤ 0.1 pp and no verdict is affected.
(2) A build-total rate (archive bytes / build ms) is not tabulated; the
directly measured rates (emission, end-to-end) and the CRC rate are, and
every segment rate is derivable from the quoted ms and byte counts.

## R3 — The 013 decision, stated as a gate: PASS

The gate is ≥ 15% of end-to-end on at least two recipes at 200k on agreeing
reads (`specs/012-phase-bench/SPEC.md:92`, `TESTING.md:811-813`). The entry
splits build − emission by the anchor before gating, which is the correct
reading: each fusion is gated on the share it would actually remove, not on
the whole segment.

- (i) Emit-into-archive — **no-go, supported.** Its removable share is the
  copy + staging remainder: 2.2–2.3% on the claimed floats 200k pair,
  2.9–3.1% († ints), 2.6–3.0% (‡ null), ~2.7–2.9% (‡ three-sheet), 2.0–2.1%
  on claimed floats 20k (`TESTING.md:820-826`). An order under the gate on
  every denominator that exists.
- (ii) Incremental CRC during emission — **no-go, supported.** CRC share of
  end-to-end: 8.5% claimed floats 200k, 14.3–14.4% † ints, 13.5% ‡ null,
  11.0% ‡ three-sheet. At the most favourable edge of the anchor and read
  ranges (highest predicted CRC over the lowest ints read) I re-derive
  14.47% — still under 15%, as the entry states (`TESTING.md:826-831`). No
  recipe reaches the line, let alone two; and strictly, only the floats
  denominator is a claimed agreeing pair, where the share is 8.5%.
- (iii) Neither — **stands.** Emission owns 54–73% at 200k on every recipe,
  so 013's target is the emitter (`TESTING.md:832-836`). The completeness
  note is also correct: the two fusions bundled would remove all of
  build − emission (17.2–17.5% † / 16.1–16.5% ‡), over the line only as a
  bundle and only off unclaimed denominators; the spec gates the candidates
  individually (`TESTING.md:836-841`). The file-write phase (15.6–16.1%
  claimed floats, up to ~29% †) is recorded for the next spec author and
  correctly not acted on, since it is not an R3 candidate
  (`TESTING.md:841-845`).

The producer's conclusion — both 013 fusions no-go — is supported by the
shares on the claimed reads alone and corroborated by every †/‡ range.

## R4 — Peak-memory observation, one probe: PASS

The entry quotes method (whole-process peak RSS via `/usr/bin/time -l`,
three runs each, bytes held alive to exit), the 200k three-sheet recipe,
and the figures: `build_workbook` 47611904–47628288 bytes, end-to-end
47628288 bytes, double residency 21334138 + 21338118 = 42672256 live bytes
(`TESTING.md:793-809`). I re-ran the probe shape from that description (a
`/tmp` scratch crate against `thoth` by path, release profile, same
recipe): sheet bytes reproduced exactly (10466925 + 10866926 + 287 =
21334138; archive and file both 21338118), build peak RSS read 47611904 on
all three runs, write peak read 47611904 once and 47628288 twice — inside
the quoted band, with the file write adding at most one 16 KB page. The
claim "the peak is set inside the build" holds. Internal consistency also
checks: the archive overhead over sheets-sum is 3980 bytes at both the 20k
and 200k three-sheet scales, as identical sheet/part name lengths require.
The probe stayed in `/tmp` scratch; nothing probe-shaped is in-tree.

## R5 — Byte-identity, library untouched: PASS (baseline reading recorded)

`sh check.sh`, run by this reviewer, ends "all gates green" (fmt check,
tests, clippy as errors, docs). The `src/` diff in this tree is the
uncommitted 011 multi-sheet work (fragment statics, `Sheet`,
`build_workbook`/`write_workbook_file`); nothing in it is phase-bench code,
and the `phase` group only calls the pre-existing public `sheet_xml`
(`src/xlsx.rs:498`) and `build_workbook` (`src/xlsx.rs:568`). 012's own
footprint is the one new bench group and the Post-012 entry, as scoped.
The spec's literal R5 test (`git diff --stat src/ tests/` empty) cannot hold
against HEAD while 011 itself sits uncommitted on the same stack; the
entry's restatement — `src/` byte-identical to its pre-012 state
(`TESTING.md:875-878`) — is the only coherent reading, and under it R5
passes: the opening and closing diff hashes for this review are identical
(see header), so this review itself changed nothing outside this file.
Byte-identity of output rests on the green round-trip, verbatim-embed,
golden, and pin tests in the gate plus the entry's `/tmp` byte probe
matching the 003 and Post-011 figures (`TESTING.md:866-879`); the R4 probe
above independently reproduced the 200k sheet and archive byte counts.

## Independent spot re-run

Direct binary, single filter, `--bench` flag, per the TESTING.md protocol:
`./target/release/deps/fill_write-ec72a5f783e16fae phase/emit_tall_floats_200000 --bench`.
The estimates file moved (Oct 12 21:49:16 → 22:04:17) before quoting. New
read: 8.6446 ms [8.6402, 8.6492], 3% outliers, against the recorded pair
8.6057/8.6158 ms — +0.4%, criterion calls it within noise. The recorded
phase numbers are reproducible in order of magnitude on this box (load
~1.5–2.2 at re-run, against the entry's 1.8–3.8 spell range).

## Required changes

None. 013 may proceed on the recorded gate: both fusions no-go, scope
shrunk to the emitter, with the shrink to be recorded in 013's
`DECISION.md` by its applicator.
