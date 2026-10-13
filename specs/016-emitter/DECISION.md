# Cycle 016 — Decision (emitter)

Applicator, after the reviewer. Machine: Darwin 27.0.0 arm64
(`RELEASE_ARM64_T8132`), Apple M4, rustc 1.93.1, bench profile
(`overflow-checks = true`, `codegen-units = 1`), HEAD `3c98e3b`
plus the uncommitted 016 work. Everything left uncommitted.

## Agreement with the review

Reviewer verdict: **Pass. No required code changes.** I agree
with the review in full; no disagreement is recorded, because
every finding is tied to inspected evidence and my own gates
reproduce it:

- The tree change is exactly the C1 test-pin extension in
  `src/emit.rs` plus the Post-016 entry in `TESTING.md`; no
  emitter spelling shipped (C1–C3 reverted, C4 refused).
- The drivers, the out-of-scope files, and the public
  signatures are untouched (R3 below).
- Byte-identity holds on the reviewer's own HEAD-dump `cmp`s
  and is entailed by the diff shape (R1 below).

No code, test, or `TESTING.md` change is applied by the
applicator. The only applicator edit is this `DECISION.md`,
inside the SPEC scoped list (`specs/016-emitter/` records by
role). `src/emit.rs`, `src/contract.rs`, `TESTING.md`, and the
untouched list were not edited by the applicator; `cargo fmt`
was re-run and left them byte-identical (see closing record).

## Per-candidate verdicts (final state)

- **C1 — float append shape: REVERTED, pin kept.** 512-byte
  stack scratch, one arena append (SPEC `SPEC.md:30-37`).
  Producer interleaved floats build at 200k: HEAD 9.195ms vs
  candidate 10.185ms, **+10.8%** (`TESTING.md:1105-1120`).
  Reviewer reconstruction (same spelling, `/tmp` copy): dump
  `cmp`-identical, HEAD 9.095ms vs candidate 10.683ms,
  **+17.5%** (5×15, medians, `REVIEW.md:112-118`). Loser
  direction corroborated; no float spelling ships. The
  extended `byte_sink_matches_string_oracle` pin is kept and
  green on the incumbent: the sole `src/emit.rs` diff hunk
  extends the float vec at `src/emit.rs:365-372` (`1e308`,
  `-1e308`, `1.5e300`, `2.225073858507201e-308`, `1e-310`,
  `123_456_789.123_456_79`); the assertion
  (`bytes == oracle.as_bytes()`, `src/emit.rs:382`) is
  unchanged, so the sweep is extended in coverage, not
  altered in meaning.
- **C2 — blank path: REVERTED.** Shared `"<c r=\""` + letters
  prefix kept single; only the blank tail fused (row digits +
  `"\"/>"`, 16-byte tail, 4 → 3 appends), against the incumbent
  blank arm (`src/emit.rs:251-252`, `:267-268`). Producer
  interleaved null build: HEAD 4.518ms vs candidate 4.814ms,
  **+6.6%** (`TESTING.md:1121-1132`), outside the ~1–3%
  Post-014 deficit band, wrong direction. Reviewer
  reconstruction: null dump `cmp`-identical, HEAD 4.367ms vs
  candidate 4.886ms, **+11.9%** (`REVIEW.md:120-131`). Loser
  confirmed; the shape is distinct from the rejected 24-byte
  whole-cell and branch-first shapes (`SPEC.md:19-22`), and
  the null deficit stands as recorded in Post-014.
- **C3 — oracle digit respelling: REVERTED.** `push_magnitude`
  (`src/emit.rs:76-99`, digit-by-digit `push(b as char)` at
  `:96-98`) respelled as one `from_utf8` slice append, sweep
  unchanged, build path untouched (`src/emit.rs:207` already
  appends once). Producer interleaved oracle emission on ints:
  HEAD 3.902ms vs candidate 4.777ms, **+22.4%**
  (`TESTING.md:1133-1143`). Reviewer reconstruction: ints dump
  `cmp`-identical, HEAD 3.851ms vs candidate 4.770ms,
  **+23.9%** (`REVIEW.md:133-146`), reproducing the producer's
  figure to within a point. The oracle keeps its digit-by-digit
  spelling and stays the parity benchmark.
- **C4 — `unsafe` cursor: REFUSED, no candidate entered.**
  After C1–C3 every safe spelling measured slower than its
  incumbent, so no remaining candidate names the nanoseconds
  it buys over the best safe spelling, and none enters
  (`SPEC.md:50-56`, cycle-004 precedent). No `unsafe` is in
  the diff; the only `unsafe` in `src/` remains the
  pre-existing CRC intrinsics (`src/zip.rs:101-104`), outside
  this cycle's scoped files. Refusal recorded here as R3/C4
  require, consistent with `TESTING.md:1144-1150` and
  `REVIEW.md:148-158`.

No rejected shape was retried (C2 keeps the shared prefix
single; C1 changes only the append shape around the settled
`Display` spelling, `SPEC.md:23-26`), and no measured no-op
ships (STYLE rule 9).

## R3 — Drivers untouched: forced call-site list is empty

`git diff HEAD -- src/contract.rs` is empty in the final tree;
sha256 `99b020c8…` matches HEAD exactly (applicator re-checked).
`emit_sheet` (`src/contract.rs:366`) and `emit_sheet_arena`
(`src/contract.rs:410`) keep their structure, loop shape, and
per-row CRC chunking (`src/contract.rs:429-460`) by identity.
Because no spelling shipped, no signature changed and no
call site was forced:

- **Forced call-site changes: none.**
- **Public signature changes: none.**
- **Scoped/untouched check (applicator re-checked, empty
  diffs):** `src/xlsx.rs`, `src/workbook_parts.rs`,
  `src/zip.rs`, `src/macros.rs`, `src/static_parts.rs`,
  `Cargo.toml`, `Cargo.lock` all unchanged; no new
  dependencies. Only `src/emit.rs` (test-pin extension) and
  `TESTING.md` (Post-016 entry) carry the producer's work,
  plus `specs/016-emitter/REVIEW.md` (reviewer) and this file.

## R1 — Byte-identity (final state)

Reviewer's own runs, not the producer's dumps: fresh
`git archive` HEAD extraction (`/tmp/thoth-head-review`) vs
the final tree, `build_workbook` dumps at 200,000 rows,
`cmp` HEAD vs final identical on every SPEC recipe
(`SPEC.md:60-64`) — ints 10470009, floats 10870010, null
10470071, mixed 17047851, three-sheet 21338118
(`REVIEW.md:27-36`) — and byte-equal to the producer's
`/tmp/dumps-head` set. Applicator corroboration: the final
`git diff HEAD` is `TESTING.md` (+79) and `src/emit.rs` (+8,
test-only hunk above); no emitter logic, driver, or template
code differs from HEAD, so no emitted byte can differ. The
emitter sweeps pass unmodified in what they assert
(`int_spelling_matches` `src/emit.rs:307` untouched;
`byte_sink_matches_string_oracle` extended as above).

## R2 / R4 — Parity and Post-016 record

Method as recorded (`TESTING.md:1095-1103`): interleaved A/B,
rotating order, seven rounds of 15, median of round medians,
byte-identity asserted before timing; no sequential Criterion
close-call evidence. Reviewer corroboration (5×15, rotating
order, medians, `REVIEW.md:92-99`): build floats HEAD 8.960ms
vs final 9.048ms (+1.0%), ints 3.929 vs 3.836ms (−2.4%), null
4.364 vs 4.374ms (+0.2%); oracle emission ints 3.796 vs
3.704ms (−2.4%), floats 9.265 vs 9.259ms (−0.1%) — overlapping
parity, matching the Post-016 final reads and the Post-015
baselines (ints 3.83–3.92ms, floats 8.90–8.93ms, null
4.45–4.52ms; `SPEC.md:11-13`). `TESTING.md:1090-1167` states
per-candidate verdicts with numbers, machine, profile, and
commit, and claims speed per candidate or not at all — here,
not at all, as no emitter code shipped. The reviewer's
non-blocking observation (C1 non-target reads on three rounds,
`TESTING.md:1113-1115`) carries no ship decision and needs no
re-take.

## Applicator's own runs (closing record)

- `cargo fmt`: applied, exit 0; `cargo fmt --check` clean
  afterwards. No file changed by the format run (`git status`
  unchanged: `M TESTING.md`, `M src/emit.rs`,
  `?? specs/016-emitter/REVIEW.md` before this file).
- `sh check.sh`: **all gates green**, exit 0 — format check,
  33 lib tests (including `byte_sink_matches_string_oracle`
  and `int_spelling_matches`) plus integration suites,
  clippy `-D warnings`, docs.
- Scoped files only: the applicator edited only
  `specs/016-emitter/DECISION.md`. Everything is left
  uncommitted, as instructed.
