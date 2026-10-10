# Spec cycles: producer -> reviewer -> applicator

Every change goes through three separate agents from one written spec.
Agents stay narrow: each brief carries only what its role needs, never
the full discussion history.

- **Producer.** Implements the cycle's `SPEC.md` and nothing else.
  Minimal diff, repo conventions honored. Does not review its own work.
- **Reviewer.** Reads code and spec, builds, runs probes and tests.
  Cannot edit source: the tree must be byte-identical before and after
  (verified by hash). Writes `REVIEW.md` with findings, each tied to an
  inspected file and line.
- **Applicator.** Makes the required code changes. May disagree with
  the review, but records the disagreement and its reasoning in
  `DECISION.md` before changing code. Writes `DECISION.md` either way.

## Per-cycle layout

`specs/NNN-slug/` holds the whole record:

- `SPEC.md` — written up front. The only instruction the producer gets.
- `REVIEW.md` — written by the reviewer. Findings plus build and test
  evidence. No code changes.
- `DECISION.md` — written by the applicator. What changed, what was
  rejected and why.

Number cycles sequentially (`001`, `002`, …) with a short slug. Only in
exceptional, explicitly approved cases does the top-level agent act as
producer.

## Binding design reference

This project is deliberately far from idiomatic Rust practice, so agent
defaults do not apply here. `STYLE.md`, `DESIGN.md`, and `TESTING.md`
at the repo root are binding on every agent in every cycle, ahead of
any idiom or best practice the agent would otherwise reach for. Every
brief includes them as required reading alongside the cycle `SPEC.md`.
Any deviation an agent believes is warranted must be recorded, never
applied silently: the reviewer flags it in `REVIEW.md`, the applicator
records it in `DECISION.md`.
