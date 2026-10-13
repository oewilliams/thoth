# Cycle 018 — styles (bold and fill colour through the contract)

DESIGN Scope promises cells may be bold, coloured, or both,
from a small fixed palette; only the header's bold exists.
This cycle adds per-column styles through the template
contract (015), the macro grammar whose style brackets were
reserved in the UX pass, and both emission drivers. No
per-cell styles, no conditional formatting: style is declared
once per column at the template, like kind.

Depends on 015 (contract), 017 (all four kinds), and the
boundary framing: past validation the emitters spell bits,
and a style index is one more declared fact about the column.

## Decisions pinned before the spec

- **Colour means cell background (fill), never font colour.**
  Bold is the font axis; colour is the fill axis; the two
  combine orthogonally. Settled with the human before this
  spec was written, with the two alternatives (font colour,
  both spellings) refused for this cycle.
- **The palette is six fixed colours**, in this order, with
  these `aRGB` fills (alpha `FF` throughout): Red `FFFF0000`,
  Green `FF00B050`, Amber `FFFFC000`, Blue `FF0070C0`,
  Purple `FF7030A0`, Grey `FFBFBFBF`. No caller-supplied hex,
  no theme colours, no new colour without a spec cycle.
- **The header row is unchanged.** Headers keep today's
  bold-only spelling (`s="1"`) whatever the column style;
  styles apply to data cells only. Settled with the human.
- **Style is per column, stated once.** A column declares
  `bold` and/or one palette colour. Every data cell in the
  column — blank cells included, so a blank in a coloured
  column still shows the fill — carries the style. Splitting
  a column is how a caller gets two styles; the writer never
  decides style per cell.

## The styles part (frozen, extended)

`xl/styles.xml` stays one frozen whole part. Indices 0 and 1
do not move, so every existing byte in every sheet survives:

- Fonts stay two: `0` plain, `1` bold.
- Fills `0` (`none`) and `1` (`gray125`) stay; fills `2..=7`
  are appended in palette order, each a solid fill spelling
  its colour as the foreground: `patternType="solid"` with
  `fgColor rgb="..."`. The exact element spelling is the
  producer's to fix once and pin; it must be the spelling
  Excel opens without repair.
- `cellXfs` keeps `0` (plain) and `1` (bold, header and
  bold-only data cells share it), then appends one plain and
  one bold entry per colour in palette order. The style
  index of a declared style is therefore arithmetic, computed
  once per column, never looked up per cell:

  | declared style | `cellXfs` index |
  |---|---|
  | plain (not bold, no colour) | none — no `s` attribute |
  | bold only | `1` |
  | colour `i` (0-based, palette order), not bold | `2 + 2i` |
  | colour `i`, bold | `3 + 2i` |

  Coloured entries carry `applyFill="1"`; bold entries carry
  `applyFont="1"` (bold-colour entries carry both). Counts
  in the part (`fonts`, `fills`, `cellXfs`) must match the
  elements actually spelled. The pin test in
  `src/workbook_parts.rs` trips on the old bytes by design:
  recapture it from the new part and record the new length
  and CRC in the Post-018 entry, as prior part changes did.

## Requirements (each item testable)

- **R1 — Style types through the contract.** A `Colour`
  enum (the six palette variants, in palette order) and a
  `Style` value (bold flag plus optional colour, plain by
  default) in `src/contract.rs`. `Column`, every
  `StaticColumn` variant, and `ColumnView` each carry the
  declared style, gathered once per sheet like kind. Plain
  construction stays one obvious spelling (`Style::default()`
  or a named plain constant — producer picks one and uses it
  everywhere). Kind-mismatch and all other boundary behaviour
  are unchanged; style needs no validation (every `Style`
  value is representable in the styles part by construction).
- **R2 — Emission spelling.** Both drivers spell the style
  as an `s` attribute on the data cell's `<c>` element,
  placed immediately after the `r` attribute and before any
  `t` attribute: numeric `<c r="A2" s="5"><v>…`, bool
  `<c r="C3" s="5" t="b">…`, string
  `<c r="D4" s="5" t="inlineStr">…`, blank
  `<c r="A2" s="5"/>`. Plain columns emit no `s` attribute
  at all. Blank cells in styled columns carry the index
  exactly as valued cells do. Headers are byte-identical to
  HEAD in all cases. The per-column index (or its pre-spelled
  attribute fragment) is computed once per sheet per column,
  never formatted per cell — the bench in R7 is the check
  that this costs nothing measurable on the plain path.
- **R3 — One spelling per driver pair.** Oracle (`String`)
  and arena (bytes) styled spellings are byte-identical to
  each other, pinned by extending the emitter sweep tests
  over: every kind valued and blank, bold-only, every colour
  plain and bold (the full cross at least once, not per
  kind × colour). Unstyled bytes do not change: HEAD dumps
  at 200k on ints, floats, null, mixed, strings/bools, and
  three-sheet compare identical (the styles part itself
  grows; sheet bytes and all other parts do not).
- **R4 — Sizing learns styles.** `sheet_cap` stays a true
  worst case with styled columns: the widest `s` spelling
  (`s="13"`, seven bytes with the leading space) priced per
  data cell in a styled column, blanks included, proved by
  extending `sheet_cap_holds` to styled columns of every
  kind. `sheet_hint` prices the column's exact index
  spelling per data cell, so styled sheets reserve near
  their actual size; retained capacity per recipe is
  recorded in the Post-018 entry.
- **R5 — Macro grammar activates the reserved brackets.**
  In `static_sheet!`, `dynamic_sheet!`, and `columns!` the
  bracket after the kind accepts `bold`, one palette colour
  name (`red`, `green`, `amber`, `blue`, `purple`, `grey`),
  or a colour and `bold` in either order (`[red, bold]`,
  `[bold, red]`). Sugar only: each expands to the plain
  `Style` value a user could write by hand. An unknown style
  token, an unknown colour, two colours, or a repeated
  `bold` fails at compile time naming the style grammar —
  never a silent plain column. The existing compile-fail
  expectation for `[bold]` is replaced by acceptance tests
  (macro expansion equals the hand-written styled value byte
  for byte) plus compile-fail expectations for the refused
  spellings where the harness can state them.
- **R6 — Callers and round-trip.** A caller exercising the
  styles (an example, beside `examples/strings_bools.rs`):
  at least one bold, one colour, and one bold-colour column
  across mixed kinds, written through the macro grammar.
  Values round-trip unchanged through the independent
  reader (styles do not touch values). Style correctness is
  asserted on the bytes: tests extract the sheet part and
  `xl/styles.xml` through the existing stored-zip test path
  and assert the `s` indices per cell (valued and blank),
  the palette fills, and the `cellXfs` table. The test
  store gains the reserved bold/colour example from
  `TESTING.md`'s reserved list, with styles carried in the
  example's column declarations.
- **R7 — Measured record.** Post-018 `TESTING.md` entry:
  existing recipes shown unregressed on agreeing or
  interleaved reads (the plain path must not pay for the
  style branch), one styled recipe at 200k (mixed kinds,
  every column styled) priced against its unstyled twin so
  the cost of the `s` bytes themselves is stated, machine,
  profile, and commit stated. No speed claim beyond the
  reads. The 017 null-timing discrepancy stays unquoted
  unless a within-binary probe settles it; this cycle does
  not have to, but must not make it worse by quoting
  cross-binary null numbers.

## Scoped files

`src/contract.rs` (`Colour`/`Style`, column and view
carriage, cap/hint, both drivers), `src/emit.rs` (styled
cell spellings and their byte twins), `src/static_parts.rs`
(the extended `STYLES`), `src/workbook_parts.rs` (pin
recapture), `src/macros.rs` (style brackets), `src/lib.rs`
(the crate doc's "no style object" line is stale once a
`Style` type ships — correct it), `examples/` (styled
caller), `tests/` (store example, byte assertions,
round-trip), `benches/` (styled recipe beside its twin),
`DESIGN.md` (Scope wording updated by the applicator to
as-shipped), `TESTING.md` (Post-018 entry),
`specs/018-styles/` records by role. No new dependencies.

## Boundaries (not this cycle)

- No per-cell styles, no conditional formatting, no font
  colours, no caller hex values, no number formats, no
  column widths or freeze panes. The header's style is not
  declarable.
- No emitter retuning beyond the `s` fragment, no `unsafe`,
  no driver restructuring. The blank-heavy hint question
  (recorded 1.93× retention) is not decided here; the hint
  change in R4 prices only the new `s` bytes.

## Workflow (on `go`)

Producer, reviewer, applicator as three independent agents,
one role each, per `specs/README.md`. The producer gets this
SPEC plus `STYLE.md`, `DESIGN.md`, `TESTING.md`, and nothing
else. Everything left uncommitted.
