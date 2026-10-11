# Cycle 010 — REVIEW (harness harden SPEC)

Reviewer read in full: `specs/010-bench-harness/SPEC.md`, `benches/fill_write.rs`,
`examples/bench.rs`, `tests/common/mod.rs`, `STYLE.md`, and the `TESTING.md`
benchmark protocol plus Post-005/Post-007/Post-009 entries (via targeted reads).
No file outside this review was modified.

## Cited-body verification (reviewer checked, not taken on trust)

- `benches/fill_write.rs:26-34` (`write_iter`): timed closure holds exactly
  `write_path` (pure path build, no fs), `remove_file` (fs), `writer_columns()`
  (clone + `Vec` build), `write_xlsx_file`, `remove_file` (fs). SPEC description
  accurate.
- `examples/bench.rs:13-28` (`bench`): `remove_file`, then `Instant::now()`,
  then `writer_columns()` + `write_xlsx_file` inside the clock. SPEC description
  accurate.
- `tests/common/mod.rs:31-51` (`writer_columns`): per-column `name.clone()` plus
  the collecting `Vec` — allocator work owned by the example store, not by the
  shipped library path. It is harness scaffolding, not library fill.
- Baselines quoted in R5 all match `TESTING.md`: ints Post-005 10.18ms / Post-009
  8.248ms; floats Post-009 12.553ms; floor 10.062ms with agreeing 10.108ms;
  byte sizes 48001 / 50002, 990005 / 1030006, 10470009 / 10870010; same-binary
  spell span 8.25–10.64ms = 2.39ms arithmetic correct.
- R4 suite names exist: `examples_round_trip_through_disk` (`tests/roundtrip.rs:17`),
  `golden_small_archive_is_byte_identical` (`src/zip.rs:316`),
  `static_parts_are_pinned` (`src/xlsx.rs:484`), `sheet_cap_holds` (`src/xlsx.rs:510`).
- Protocol details R5 invokes (single `--bench` filter flag, estimates-mtime check,
  bench profile `overflow-checks = true` + `codegen-units = 1`) are present in the
  `TESTING.md` benchmarks section (lines ~53–65). `STYLE.md` rules 8/9 citation correct.

## R1 — Setup leaves the timed section: APPROVE

The timed-section contract today is "fill plus write" (`TESTING.md`, `RUNOFF.md`,
both bench doc comments). The contamination is real and correctly itemized:
two `remove_file` syscalls and the column clone+`Vec` build ride the clock every
iteration. Moving all three into `iter_with_setup` (or equivalent) setup preserves
the workload: `write_xlsx_file` receives byte-identical arguments either way, and
per-iteration setup means no cross-iteration caching or aliasing — same work, less
clock. Groups, recipes, scales, and criterion version are explicitly frozen, so
there is no scope for a recipe change to hide inside the harness edit.

Ruling on the SPEC's open conditional: `writer_columns()` is **setup, not fill**.
It is store-to-writer adaptation (the clone exists because the example owns its
`Vec`s), while production callers supply columns directly. Both harnesses therefore
time the `write_xlsx_file` call alone. The absolute difference is negligible
(two short-string clones against 200k rows), so this ruling changes no number
materially — it just makes the contract statable in one sentence, identically for
both harnesses. The producer names the timed lines in the Post-010 entry; I confirm
the target shape is `write_xlsx_file`-only in both.

Mechanism note for the applicator: criterion 0.5 has no post-iteration teardown
hook in `iter_with_setup` — pre-cleaning in setup plus one trailing cleanup after
the group satisfies "teardown outside the clock". Debris must never be deleted
inside the timed closure.

## R2 — Spell-agreement rule: APPROVE

Falsifiable, not vibes. Every threshold names a quantity criterion prints:
pairwise mean agreement within 3%, CI half-width under 5% of the mean, severe-outlier
share at or under 10%, same binary. Each is checkable from the criterion report;
a number recorded without two quoted agreeing reads, or with a discarded pass
missing figures, fails the requirement on its face. The discard-documentation
standard (figures + supersede note, Post-006/007/009 precedent) makes silent
cherry-picking a detectable violation rather than a temptation. Writing the rule
into `TESTING.md` promotes it from cycle lore to standing protocol, which is the
right home (measurement definition per `STYLE.md` rule 9).

Two tightenings, neither blocking: (a) "same spell" needs an operational reading —
back-to-back runs in one session with box state re-checked between them (R3 gives
the instruments); a "spell" interrupted by a load spike is a new spell. (b) The 3%
bar is stricter than the 5–10% steady-state spread, so the failure mode is producer
churn (many discarded spells), never false acceptance. That asymmetry is correct
for this cycle; if no spell ever qualifies, that is evidence about the box, not a
license to loosen the rule mid-cycle.

## R3 — Box state recorded: APPROVE

Uptime, 1-minute load, and free pages (with Darwin and Linux spellings) are cheap,
objective, and directly implicated by the Post-009 diagnosis (42-day uptime,
transient busy spells). Recording them beside each 200k figure makes the next
"machine spread, not code effect" claim checkable instead of asserted. Small scales
are rightly exempt — the cost/benefit only pays at 200k. Test is mechanical
(re-run the stated commands, confirm the shape).

## R4 — Byte-identity, suite green unmodified: APPROVE, sufficient

Sufficient with margin. The harness files under change (`benches/`, `examples/`)
are not linked into the shipped library artifact, so byte-identity risk is nil by
construction; the suite is belt-and-braces, and it is the right belt: round-trip
values, stored-zip verbatim-embed, golden archive, static pins, and the 003
byte-size probe (figures verified above) together detect any output change down to
one byte. Prior cycles (007, 009) used exactly this proof shape. The
`git diff --stat tests/ src/` emptiness test pins the "unmodified" half.

## R5 — 200k re-take, spread narrowed: APPROVE

Testable end to end: three named benches, fixed profile and invocation (scored
harness, `--bench` single filter, mtime check — all three address documented past
footguns), absolute figures with machine/profile/commit, both agreeing reads per
bench, both baselines named, and pass/fail arithmetic on the span (under 1.0ms vs
the 2.39ms Post-009 span). The no-speedup guardrail ("same speed, tighter clock"
unless the agreeing reads sit outside the old interval) correctly inherits the
Post-009 labeling of −19.0% as machine spread. Re-taking the floor control
alongside the two cell benches preserves the formatting-cost isolation the floor
exists for.

## Scope / boundaries: APPROVE

Scoped files admit exactly the harness shape, clock lines, `TESTING.md` rule text
plus Post-010 entry, and role records. `src/`, `tests/`, `Cargo.toml`/`Cargo.lock`,
`STYLE.md`, `DESIGN.md` frozen; no new tests, benches, or dependencies; probes in
`/tmp`. The "one changed byte fails the cycle" acceptance line is the correct
strength for a no-library-change cycle. Nothing in the SPEC authorizes a speedup
claim from the harness change alone.

## Overall: APPROVE, no rework

The SPEC is testable per requirement, its cited bodies and figures are accurate as
verified above, and its three load-bearing judgments survive inspection: the clock
covers fill-plus-write only with identical workload, byte-identity proof is
sufficient, and the spell-agreement rule is falsifiable. Applicator inherits two
rulings: (1) both harnesses time `write_xlsx_file` alone; (2) "same spell" means
consecutive runs with box state re-checked. Reviewer artifacts: this file only;
tree otherwise untouched.
