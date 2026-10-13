# Cycle 016 — Review (emitter)

Reviewer, independent of the producer. Snapshot before review:
`git status` showed only `M TESTING.md` and `M src/emit.rs`
(per-file sha256 set recorded in `/tmp/review016-before-hashes.txt`,
diff sha256 `f8d6b227…`); `specs/016-emitter/` held only `SPEC.md`.
This review adds only this file (verified at the end). Machine:
Darwin 27.0.0 arm64 (`RELEASE_ARM64_T8132`), Apple M4, rustc 1.93.1,
bench profile (`overflow-checks = true`, `codegen-units = 1`),
HEAD `3c98e3b` plus the uncommitted 016 work — same statement as
the Post-016 entry (`TESTING.md:1090-1094`).

## Overall verdict

**Pass. No required code changes.** No emitter spelling shipped:
C1–C3 were each measured slower than the incumbent on interleaved
A/B and reverted, C4 was refused for want of a nanosecond figure,
and the only shipped tree change is the C1 byte-identity test pin.
My own gates, HEAD-dump `cmp`s, final-vs-HEAD interleaved reads,
and reconstructions of all three reverted candidates corroborate
the Post-016 record. The one outstanding item is applicator-stage
by workflow design: `DECISION.md` does not exist yet and must
carry the C4 refusal (see R3/C4).

## R1 — Byte-identity is absolute: PASS

Own runs, not the producer's dumps: fresh `git archive` HEAD
extraction (`/tmp/thoth-head-review`, `src/` diffed clean against
HEAD) and the final tree, each behind an identical `/tmp` probe
crate (release profile as above, columns built once outside the
clock). `build_workbook` dumps at 200,000 rows, `cmp` HEAD vs
final: ints 10470009, floats 10870010, null 10470071, mixed
17047851, three-sheet 21338118 — identical on every recipe, and
byte-equal to the producer's `/tmp/dumps-head` set. These are the
SPEC recipes (`SPEC.md:60-64`) at the Post-015 lengths
(`TESTING.md:1018-1019`).

Emitter sweeps pass unmodified in what they assert: `sh check.sh`
green on my run (fmt check, 33 lib tests, clippy `-D warnings`,
docs), including `int_spelling_matches` (`src/emit.rs:307`,
untouched by the diff) and `byte_sink_matches_string_oracle`
(`src/emit.rs:336`). The sole diff hunk in `src/emit.rs` extends
that test's float vec (`src/emit.rs:365-372`: `1e308`, `-1e308`,
`1.5e300`, `2.225073858507201e-308`, `1e-310`,
`123_456_789.123_456_79`) — the assertion (`bytes ==
oracle.as_bytes()`, `src/emit.rs:382`) is unchanged, so the sweep
is extended in coverage, not altered in meaning, exactly as C1
prescribes (`SPEC.md:34-37`). The pin covers the cycle-007 set:
specials and both zeros (`src/emit.rs:351-364`), min-normal,
the subnormal floor, quarter-steps (`src/emit.rs:374-376`), and
now the 301-digit positional extremes.

## R2 — One candidate, one measurement: PASS

Method as recorded (`TESTING.md:1095-1103`): interleaved A/B,
rotating order, seven rounds of 15, median of round medians,
byte-identity asserted before timing; no sequential Criterion
close-call evidence. Each loser was reverted — the final diff
contains no candidate spelling (see C1–C3) — so no measured
no-op ships (STYLE rule 9). No rejected shape was retried: C2
keeps the shared `"<c r=\""` + letters prefix single and fuses
only the blank tail, which is neither the 24-byte whole-cell
stack assembly nor the branch-first duplicated open
(`SPEC.md:19-22`; `TESTING.md:1121-1126`). C1 changes only the
append shape around the settled `Display` spelling, not the
spelling itself (`SPEC.md:23-26`). My spot re-runs below
re-derive the loser direction for all three candidates.

## R3 — Drivers untouched: PASS (record pending, applicator-stage)

`git diff HEAD -- src/contract.rs` is empty; sha256
`99b020c8…` matches HEAD exactly. `emit_sheet`
(`src/contract.rs:366`) and `emit_sheet_arena`
(`src/contract.rs:410`) keep their structure, loop shape, and
per-row CRC chunking (`src/contract.rs:429-460`) by identity,
not by resemblance. No shipped spelling means no forced
call-site change and no signature change; scoped-file check:
`src/xlsx.rs`, `src/workbook_parts.rs`, `src/zip.rs`,
`src/macros.rs`, `src/static_parts.rs`, `Cargo.toml`,
`Cargo.lock` all unchanged, no new dependencies. The
`DECISION.md` listing R3 requires therefore has an empty
call-site list to record — but the file itself is the
applicator's, per `specs/README.md`; its absence is not a
producer defect. **Applicator must record in `DECISION.md`:
no forced call sites, no signature change, and the C4 refusal.**

## R4 — Post-016 record: PASS

`TESTING.md:1090-1167` states per-candidate verdicts with
numbers, final reads against the Post-015 baselines, machine,
profile, and commit, and claims speed per candidate or not at
all — here, not at all. My corroboration (reviewer probes,
5 rounds × 15, rotating order, medians): build floats HEAD
8.960ms vs final 9.048ms (+1.0%), ints 3.929 vs 3.836ms
(−2.4%), null 4.364 vs 4.374ms (+0.2%); oracle emission ints
3.796 vs 3.704ms (−2.4%), floats 9.265 vs 9.259ms (−0.1%) —
all overlapping-round parity, matching the entry's parity
reading (`TESTING.md:1157-1165`) and the Post-015 baselines
(ints 3.83–3.92ms, floats 8.90–8.93ms, null 4.45–4.52ms;
`SPEC.md:11-13`). Non-blocking observation: C1's non-target
reads rest on three rounds (`TESTING.md:1113-1115`); they are
context for a verdict already decided on the target recipe and
carry no ship decision, so no re-take is required.

## C1 — Float append shape: REVERTED, honest

Spec shape (`SPEC.md:30-37`): 512-byte stack scratch, one arena
append, against the write-through incumbent
(`src/emit.rs:263-277` via `VecFmt`, `src/emit.rs:162-169`).
Recorded: byte-identical everywhere; interleaved floats build
9.195ms vs 10.185ms, **+10.8%** (`TESTING.md:1105-1120`).
Reviewer reconstruction (same spelling in a `/tmp` copy, one
`extend_from_slice` from the scratch): floats dump `cmp`-
identical to HEAD, interleaved build HEAD 9.095ms vs candidate
10.683ms, **+17.5%** (5×15, medians) — same loser direction,
larger in my spelling; the revert is honest and no float
spelling shipped. The extended pin is kept and green on the
incumbent (R1 above), as the spec directs.

## C2 — Blank path: REVERTED, honest

Candidate as named (`TESTING.md:1121-1132`): shared prefix kept,
blank arm fuses row digits + `"\"/>"` into one append from a
16-byte tail, 4 → 3 appends, against the incumbent blank arm
(`src/emit.rs:251-252`, `:267-268`). Recorded: byte-identical;
interleaved null build 4.518ms vs 4.814ms, **+6.6%** — outside
the ~1–3% Post-014 deficit band, wrong direction. Reviewer
reconstruction of the same fusion: null dump `cmp`-identical
to HEAD, interleaved build HEAD 4.367ms vs candidate 4.886ms,
**+11.9%**. Loser confirmed; the deficit stands as recorded in
Post-014, not carried.

## C3 — Oracle digit respelling: REVERTED, honest

Candidate (`TESTING.md:1133-1143`): `push_magnitude`
(`src/emit.rs:76-99`, digit-by-digit `push(b as char)` at
`:96-98`) respelled as one `from_utf8` slice append, sweep
unchanged, build path untouched (the byte spelling already
appends once, `src/emit.rs:207`). Recorded: build bytes
identical; interleaved oracle emission on ints 3.902ms vs
4.777ms, **+22.4%**. Reviewer reconstruction (same respelling,
per-char fallback on `Err`): ints dump `cmp`-identical to
HEAD, interleaved emission HEAD 3.851ms vs candidate 4.770ms,
**+23.9%** — the producer's figure reproduced to within a
point. The oracle keeps its digit-by-digit spelling and stays
the parity benchmark.

## C4 — `unsafe` cursor: REFUSED, correctly

No `unsafe` in the diff; the only `unsafe` in `src/` remains
the pre-existing CRC intrinsics (`src/zip.rs:101-104`), outside
this cycle's scoped files. After C1–C3 every safe spelling
measured slower than its incumbent, so no candidate names
nanoseconds over the best safe spelling and none enters
(`SPEC.md:50-56`, cycle-004 precedent). No `unsafe` in the
diff needs justification; none is flagged for removal. The
refusal is recorded at `TESTING.md:1144-1150` for the
applicator's `DECISION.md`, which remains the one open record.

## Required changes

None to code, tests, or `TESTING.md`. Applicator-stage record
only: `DECISION.md` must record the C4 refusal, the empty
forced-call-site list under R3, and the per-candidate verdicts
as shipped/reverted above.

Tree-identical check: after writing this file, `git status`
and the per-file sha256 set match the pre-review snapshot
except the added `specs/016-emitter/REVIEW.md`; all probes and
candidate reconstructions lived in `/tmp`.
