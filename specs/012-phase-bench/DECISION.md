# Cycle 012 — Decision (phase bench)

Applicator: independent applicator agent. Read `SPEC.md`, `REVIEW.md`,
`STYLE.md`, `DESIGN.md`, and `TESTING.md` in full before acting. The
reviewer required no changes and the applicator found no reason to
disagree, so no code, bench, test, or doc file was edited by this
application. The only file this application creates is this record.
Everything is left uncommitted.

## Review verdict and disposition

Reviewer passed R1–R5 with no required changes, on inspected code,
a green gate, an independent phase-arithmetic re-derivation, a
re-run `/tmp` peak-RSS probe, and a spot bench re-run
(`phase/emit_tall_floats_200000` 8.6446 ms against the recorded
8.6057/8.6158 ms pair, +0.4%, within noise). Disposition: accepted
in full; no disagreement to record. The reviewer's two minor,
non-blocking observations on R2 are likewise accepted as noted and
require no action this cycle: the copy-remainder ranges subtract
the CRC anchor's mid-point rather than its full range (each
remainder ~3 µs narrow per side, shares moving ≤ 0.1 pp, no
verdict affected), and no build-total rate is tabulated (every
segment rate remains derivable from the quoted ms and byte
counts).

## What shipped (producer work, as reviewed)

- **R1 — Phase recipes.** One new `phase` group in
  `benches/fill_write.rs` (registered beside the existing groups):
  emission is `sheet_xml` per sheet, summed over the workbook's
  sheets, result `black_box`ed, no file touched; build is
  `build_workbook`, no file touched; end-to-end is re-used from
  the existing `fill_write`/`cells`/`floor`/`multi_sheet`
  groups, never re-implemented. Recipes `tall_ints`,
  `tall_floats`, `tall_null` at 20,000 and 200,000 plus the 011
  three-sheet example at its bench scale; setup stays out of the
  clock and the emission allocation stays in it, exactly as the
  library performs it.
- **R2 — Phase table.** Post-012 entry in `TESTING.md` holds,
  per recipe, emission ms, build-minus-emission ms (CRC +
  assembly), write-minus-build ms (file + seal), and end-to-end
  ms as two agreeing reads with CIs, machine, profile, commit,
  and 200k box state, plus implied per-phase output rates. Every
  share is a range; both independent anchors are quoted beside
  the subtractions (the `crc` group rate predicting 78–85% of
  every build-minus-emission segment with a small positive copy
  remainder, and the `tall_null` floor bounding the float
  emission premium at ~4.5 ms of 8.61 ms at 200k). No subtraction
  disagreed with its anchor beyond the CIs, so no anchor-override
  was invoked; the †/‡ denominators (ints, null, three-sheet
  end-to-end) are demoted to corroboration, never claimed
  baselines.
- **R3 — The 013 gate.** Recorded below; this is the cycle's
  deliverable decision.
- **R4 — Peak memory.** `/tmp` scratch probe only (whole-process
  peak RSS via `/usr/bin/time -l`, three runs each, method
  named): `build_workbook` 47611904–47628288 bytes vs end-to-end
  47628288 bytes on the 200k three-sheet recipe; the file write
  adds at most one 16 KB page and the peak is set inside the
  build. The sheet-plus-archive double residency a fusion would
  remove is 21334138 + 21338118 = 42672256 live bytes. Nothing
  probe-shaped is in-tree.
- **R5 — Byte-identity.** No library byte changed by 012; see
  verification below for the baseline reading.

## The 013 gate outcome (explicit)

Gate rule (SPEC R3): a fusion is `go` only if the share it would
remove is ≥ 15% of end-to-end on at least two recipes at 200k on
agreeing reads, gated on the share each fusion would actually
remove (build-minus-emission split by the CRC anchor), not on
the whole segment.

- **(i) Emit-into-archive — no-go.** Removable share is the
  copy + staging remainder: 2.2–2.3% on the claimed floats 200k
  pair, 2.9–3.1% († ints), 2.6–3.0% (‡ null), ~2.7–2.9%
  (‡ three-sheet), 2.0–2.1% on claimed floats 20k. An order
  under the gate on every denominator that exists.
- **(ii) Incremental CRC during emission — no-go.** CRC share
  of end-to-end: 8.5% on the claimed floats 200k pair,
  14.3–14.4% † ints, 13.5% ‡ null, 11.0% ‡ three-sheet; 14.47%
  even at the most favourable edge of the anchor and read
  ranges. No recipe reaches 15%, let alone two, and only the
  floats denominator is a claimed agreeing pair.
- **(iii) Neither stands.** Emission owns 54–73% at 200k on
  every recipe (73.2–73.5% claimed floats), so the next target
  is the emitter itself — digit/float spelling and per-push
  cost — not either fusion. The completeness note also stands:
  the two fusions bundled would remove all of build-minus-
  emission (17.2–17.5% † / 16.1–16.5% ‡) only as a bundle and
  only off unclaimed denominators; the spec gates the
  candidates individually, and individually neither clears it.
  The file-write phase (15.6–16.1% claimed floats, up to ~29%
  †) is recorded for the next spec author and not acted on; it
  was never an R3 candidate.

**Therefore: both 013 fusions are no-go, cycle 013 (fused emit)
does not run, and the emitter is the next target.** This
applicator does not start 013, does not shrink or amend any 013
spec, and writes nothing in `specs/013-fused-emit/`. Any future
emitter cycle needs its own spec; any future revival of a fusion
needs a fresh measurement clearing the same 15% gate. Shipping
either fusion on these numbers would be a measured no-op
foundation change and would fail STYLE rule 9.

## Applicator verification (independent)

- **Gate:** `sh check.sh` run by this applicator in this
  session — green, ending "all gates green" (exit 0): `cargo fmt
  --check` clean, 30 unit tests + 1 round-trip + 1 multi-sheet
  + 4 sealed tests pass, `cargo clippy --locked --all-targets
  -- -D warnings` clean, docs built. A separate `cargo fmt
  --check` re-run in the same session also exited 0. Matches
  the reviewer's reported run.
- **`src/` diff:** the literal `git diff --stat src/` against
  HEAD is not empty in this tree, because the uncommitted 011
  multi-sheet work sits on the same stack (`src/static_parts.rs`,
  `src/xlsx.rs`); the reviewer's baseline reading is the only
  coherent one and this applicator adopts it explicitly: 012's
  own footprint on `src/` must be zero. Verified by hash —
  `git diff -- src/ | sha256sum` read
  `29dfb351dca25bc9752d3ec55009914f7e1ba22f63f52cfa274581bac0c9414c`
  before this application and read identically after it
  (same hash quoted above, re-run in this session after the
  gate). 012 therefore contributes zero bytes to the `src/`
  diff. No
  `src/`, `tests/`, `examples/`, manifest, `STYLE.md`,
  `DESIGN.md`, or `README.md` file was edited by this
  applicator.
- **Scope:** modified by this application: nothing. New: this
  file only. Untouched by this applicator: `benches/fill_write.rs`
  and the Post-012 `TESTING.md` entry (producer work, as
  reviewed), and everything in `specs/013-fused-emit/`.
- **Disposition:** no defect found, no disagreement with the
  reviewer, no code changed. The cycle is complete as reviewed;
  everything — the 011+012 stack, this record, and the untouched
  013 spec — is left uncommitted for the human to commit.
