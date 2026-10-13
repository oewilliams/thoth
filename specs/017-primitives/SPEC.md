# Cycle 017 — primitive kinds (strings and bools through the contract)

DESIGN Scope promises four primitives; two exist. This cycle
adds the remaining two as new kinds through the template
contract (015), the macro grammar whose `str`/`bool` tokens
were reserved in the UX pass, and both emission drivers.
Nothing else about the foundation moves.

Depends on 015 (contract), 014 (arena), and the boundary
framing: value checks at the interface, bits past it. The
emitter spellings for existing kinds are not retuned here.

## Decisions pinned before the spec

- **Nulls live in the value bits, as with the existing kinds.**
  Bool ingress is `&[Option<bool>]` (the niche is one byte,
  three states); string ingress is `&[Option<&str>]`. `None`
  spells a blank cell. No side-band masks, no sentinel strings:
  an empty string is a real value and must round-trip as one.
- **Strings are inline** (`t="inlineStr"`), per DESIGN: no
  shared-string table, no hashing. The length cap is validation,
  not an allocation strategy.
- **Excel's cell text limit is 32,767 characters**, refused at
  the boundary with column and Excel row named, never truncated.

## Requirements (each item testable)

- **R1 — Bool kind.** `Kind::Bool` and the matching data variant
  through both drivers, the static impl, and the macro grammar
  (`bool "ok" => &flags`). Spelling: `t="b"` cells with `1`/`0`
  values, `None` blank. Round-trip through the independent
  reader: the test reader learns to read bool cells back as
  bools, and a mixed-kind sheet round-trips every kind.
- **R2 — String kind.** `Kind::Str` and the matching data
  variant through both drivers, the static impl
  (`[Option<&str>; R]` data), and the macro grammar
  (`str "name" => &names`). Spelling: inline-string cells with
  the same five-entity escaping as headers; text with leading
  or trailing whitespace carries `xml:space="preserve"` on the
  `<t>` element (Excel trims without it). Control characters
  Excel cannot hold are refused at the boundary, named by
  column and row — the refusal set is the XML 1.0 valid range
  minus the whitespace Excel keeps (tab, LF, CR), spelled out
  in the code where the check lives.
- **R3 — Boundary checks for the new kinds.** String length cap
  (32,767 chars) and control-character refusal run in
  `validate_template` beside the existing checks, before any
  emission; their cost is a scan the emission would pay anyway
  to escape. Kind-mismatch refusal covers the new variants
  unchanged. Errors preserve the existing message shapes
  (column and row named).
- **R4 — Sizing learns the new kinds.** `sheet_cap` stays a true
  worst case: bool contributes its fixed spelling; string
  contributes, per cell, the escaped upper bound of the actual
  text (6x per character at most, from the escaping table) plus
  tags — data-dependent arithmetic at the boundary, with
  `sheet_cap_holds` extended to string columns. `sheet_hint`
  prices strings from witnessed lengths (text bytes plus tags),
  so string sheets reserve near their actual size; retained
  capacity per recipe is recorded in the Post-017 entry.
- **R5 — One spelling per driver pair.** The oracle (`String`)
  and arena (bytes) spellings of each new cell kind are
  byte-identical to each other, pinned by extending the emitter
  sweep tests (bools, strings with escapable characters,
  leading/trailing whitespace, empty string vs blank).
  Existing kinds' bytes do not change: HEAD dumps at 200k on
  ints, floats, null, mixed, three-sheet compare identical.
- **R6 — Measured record.** Post-017 `TESTING.md` entry: new
  recipes (tall strings at a stated text width, tall bools)
  through the phase build at 200k on agreeing or interleaved
  reads, existing recipes shown unregressed, machine, profile,
  and commit stated. No speed claim beyond the reads.

## Scoped files

`src/contract.rs` (kinds, data variants, static columns,
validation, cap/hint, both drivers), `src/emit.rs` (new cell
spellings and their byte twins), `src/xlsx.rs` (error variants
if any, kept in the existing message shapes), `src/macros.rs`
(`str`/`bool` arms; style brackets still reserved and failing),
`examples/` (a caller exercising both kinds), `tests/`
(round-trip reader support plus new tests), `benches/` (new
recipes beside the existing groups), `DESIGN.md` (Scope wording
updated by the applicator), `TESTING.md` (Post-017 entry),
`specs/017-primitives/` records by role. No new dependencies.

## Boundaries (not this cycle)

- No shared strings, no styles beyond the existing two, no
  number formats or dates (dates ride as strings or numbers,
  per DESIGN). Style markers in the macro grammar remain
  reserved syntax that fails at compile time.
- No emitter retuning of existing kinds, no `unsafe`, no
  driver restructuring.

## Workflow (on `go`)

Producer, reviewer, applicator as three independent agents, one
role each, per `specs/README.md`. The producer gets this SPEC
plus `STYLE.md`, `DESIGN.md`, `TESTING.md`, and nothing else.
Everything left uncommitted.
