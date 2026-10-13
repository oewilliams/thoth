# Cycle 015 — template contract (who declares shape, who owns emission)

The emitter cycle (016) tunes whatever interface this cycle
locks, so this cycle locks it. It defines how a sheet's shape is
declared and how emission is driven from that declaration, for
the numeric columns that exist today. Emitters are not tuned,
extended, or re-spelled here: their bytes and their cost are the
fixed point this cycle moves the callers around.

Depends on 014 (arena) committed. `DESIGN.md` Templates and the
014 locks are binding: monomorphisation is the only allowed
branch, the two template impls differ only at the template, and a
template declares a size hint from a known shape or names its
unknown dimension, known exactly before emission.

## Background

Today there is no template type. `Sheet { name, columns }` plus
the free functions are the de-facto dynamic template: borrowed
columnar slices, kinds decided per column in `contract.rs`, row
loop and cell dispatch owned by `emit_sheet` in `src/xlsx.rs`,
reservation derived inside `build_workbook`. DESIGN promises two
impls (statically allocated fixed-size workbooks; dynamic
variable-row workbooks) sharing the same monomorphic emitters,
but no static caller exists in the tree yet — every example and
bench builds its columns at runtime. Per STYLE rule 1, the static
impl therefore enters with its caller built in this cycle (an
example plus a round-trip test; a test as spec counts).

The arena cycle also left the emitter interface implicit: who
supplies column letters, row digits, per-row CRC chunking, and
the reservation hint is currently answered inside `xlsx.rs`.
This cycle makes those answers the contract's, explicitly, so
016 can tune the shared emitters against both impls at once.

## Requirements (each item testable)

- **R1 — One contract, declared once.** A sheet template declares,
  in order: its columns (name and kind per column), where each
  column's data lives, and its shape knowledge — row count, or
  that rows are unknown; column count, or that columns are
  unknown. Kind is selected per column at declaration, never per
  cell (existing rule, restated as the contract's first clause).
  The declaration is the only place shape is decided: the arena
  builder, the ceiling check, and the reservation consume it
  without re-deriving shape from the data.
- **R2 — Two impls, one emitter set.** (a) Dynamic: runtime
  columns over borrowed slices — today's behaviour, re-expressed
  behind the contract; `Sheet` either becomes this impl or is
  retired in its favour with every existing caller migrated.
  (b) Static: fixed column set and fixed row count known at
  compile time (const shape; fixed `[T; N]` data has the same
  memory shape as slices per DESIGN Ingress). Both impls drive
  the identical emitter functions; the impl choice is resolved
  by monomorphisation — no `dyn`, no per-row or per-cell branch
  on which impl is emitting (reviewer checks the hot path and,
  where the claim needs it, the assembler). The static impl's
  caller ships in this cycle: a fixed-shape example under
  `examples/` plus a round-trip test reading its output back.
- **R3 — Hint or named unknown, from the declaration.** Each impl
  produces the arena reservation through the contract: a size
  hint computed from the declared shape when known (the 014
  spelling: expected widths per kind, never the `sheet_cap`
  worst case), or the named-unknown case, which the builder
  answers with block growth (014 priced it at parity). Which
  case holds is known before the first byte is emitted. A test
  exercises the unknown-rows path end to end and compares its
  bytes against the known-shape spelling of the same sheet.
  Retained capacity per recipe is measured and recorded, as in
  Post-014; the null-heavy 1.93x retention is reported, not
  silently inherited (whether the hint learns blank-heavy
  columns is the successor's decision, named here, not taken).
- **R4 — Emission driving behind the contract.** The row loop,
  letter and row-digit supply, and per-row CRC chunk boundaries
  are driven through the contract surface, so a future emitter
  change has exactly the contract's call sites to touch.
  `src/emit.rs` is unchanged by this cycle (a diff there is a
  finding, not an edit); `sheet_xml` remains the oracle spelling.
  Error semantics are unchanged: boundary refusals (names,
  shape, ceiling) still precede all emission, value refusals
  (infinite) still surface in row order wrapped per sheet.
- **R5 — One route, and byte-identity.** Every in-tree caller —
  library entry points, examples, benches, tests — routes through
  the template contract; no parallel pre-template path survives
  (the `ZipWriter` precedent: one spelling, never two). Existing
  public signatures (`build_workbook`, `build_xlsx`,
  `write_workbook_file`, `write_xlsx_file`, `sheet_xml`) keep
  working only as spellings implemented through the contract, or
  their change is recorded in `DECISION.md` with every caller
  migrated. This widens the cycle's scope to caller migration,
  accepted deliberately: a template nobody routes through is a
  second foundation, not a contract. Archive bytes are identical
  to the pre-015 output on the 014 recipe set (ints, floats,
  null, mixed, three-sheet at 200k), probed byte for byte
  against HEAD dumps as in Post-014.
- **R6 — Measured no-regression.** Post-015 `TESTING.md` entry:
  phase build recipes and the static example's end-to-end rate
  on agreeing reads (interleaved A/B where sequential reads
  drift, per the 014 method finding), machine, profile, commit
  stated. No recipe may regress beyond noise; a regression is
  explained in the entry or the cycle does not ship. The entry
  also records what the static impl buys or does not buy in
  numbers — if compile-time shape buys nothing measurable, that
  is recorded plainly and the impl stands on ergonomics and the
  DESIGN promise, not on an invented speed claim.

## Scoped files

`src/contract.rs` (the contract and impls, or their home —
producer's shape, reviewer confirms one home), `src/xlsx.rs`
(builder and emission driving behind the contract),
`src/workbook_parts.rs` only if name validation moves behind the
contract, `examples/` (the static caller), `benches/` (caller
migration per R5), `tests/` (new template tests, plus caller
migration per R5 — a test that needs a semantic edit beyond
routing is a finding), `DESIGN.md`
(Templates wording updated by the applicator to what shipped),
`TESTING.md` (Post-015 entry), `specs/015-templates/` records
by role. Untouched: `src/emit.rs`, `src/zip.rs`,
`src/static_parts.rs`, manifests. No new dependencies, no
`unsafe`.

## Boundaries (not this cycle)

- No new cell kinds: strings, bools, styles, widths, and number
  formats arrive in later cycles as new kinds through this
  contract. No emitter tuning, digit/float respelling, skeleton
  work, or `unsafe` cursor — that is cycle 016, against the
  interface this cycle locks.
- No streaming or backpressure: unknown rows means the builder
  grows, not that data arrives incrementally; an incremental
  fill API is a later design event with its own caller.
- No public API breakage beyond R5's recorded-change clause.

## Workflow (on `go`)

Producer, reviewer, applicator as three independent agents, one
role each, per `specs/README.md`. The producer's brief carries
this SPEC plus `STYLE.md`, `DESIGN.md`, `TESTING.md` and nothing
else. Everything left uncommitted.
