# Backlog — requirements beyond the current writer

Recorded 2026-10-13, from an audit of the writer at `c25a334`
against a reference spec sheet describing a production
workbook family. The sheet itself is kept outside this
repository; what follows is the sanitised restatement —
shapes already public in `TESTING.md` are reused, nothing
else is carried over.

## Already covered (for contrast)

Multiple sheets; the four primitive kinds with inline
strings and `t="b"` bools; blanks blank; per-column bold
and fill colour from the six-colour palette; opaque
long-string payload (multi-KB cells); wide and tall shapes
to the spec family's largest sheet (8,880 rows × 1,298
columns, three sheets, built and recorded in Post-019).

## Agreed queue (in settled order)

1. **Number formats, per column.** The reference formats
   are `0.00`, `#,##0`, `0.000%`, `0.000000`, and date
   formats (`yyyy-mm-dd` for date cells, `mmm-yy` for
   date-valued headers). Every cell is General today, so
   date serials do not display as dates. This is the
   largest gap, and it multiplies the cycle 018 `cellXfs`
   table: format becomes a third declared axis beside bold
   and fill, and the index arithmetic must stay arithmetic.
2. **Header variants.** Three, all currently absent:
   date-valued headers (a numeric serial in the header row
   under a date format, not a string); a plain, unstyled
   header mode (headers are always bold today); and the
   header decoration the reference uses (thin border on all
   sides, centred/top alignment) where ours is bold font
   only.
3. **Freeze panes.** `A2`, `A4`, or none, declared per
   sheet. Sheet-view XML the writer does not emit today.
4. **Deflate — absolutely last.** With it, the
   constant-memory question: the arena holds the whole
   workbook (about 911 MB at the reference shape), while
   the reference family is written streaming. Stored versus
   deflated at that shape is about 911 MB against about
   110 MB on disk (Post-019, second run-off). Deflate
   changes stored sizes in the local headers, the CRC
   fold's feed, and the ceiling arithmetic, so it is done
   once, over a finished byte layout.

## Decisions owed (not implementation)

Each of these conflicts with a standing design position,
so each needs an explicit call before it can be specced:

- **Per-cell mixed types.** Reference columns mix float
  with a literal blank-marker string, and one sheet family
  chooses each value's type and format by its row. Kind is
  declared per column here; a mixed column is
  unrepresentable and `KindMismatch` refuses it. Admitting
  it changes the contract, not just the emitters.
- **Value-keyed status fills.** Any cell whose value is
  exactly a pass/fail token gets a status fill, per cell.
  Styles are per column here, and the two exact status
  colours are not in the six-colour palette. This is
  conditional formatting by value, which DESIGN currently
  leaves to callers precomputing it — but a per-column
  style cannot express it inside a mixed column.
- **Preamble rows and row-oriented sheets.** One reference
  sheet carries fixed preamble rows above its header and
  keys each data row's format by the row's label. The
  template model is columnar with one header row; this is
  a second sheet mode, if it is ever admitted.
- **`inf` / `-inf` as strings.** The reference writer
  spells infinities as text and must keep doing so. Here
  infinities are refused at the boundary by design. Nothing
  occurred in the reference run; the requirement is stated
  anyway.
- **Sheet-name sanitisation.** The reference maps invalid
  characters to `_`, truncates to 31, strips edge
  apostrophes, and de-duplicates with numeric suffixes.
  Here invalid or duplicate names are refused at the
  boundary by design. Sanitising is a caller convenience
  that changes names silently; refusing is the standing
  position and would need to be re-argued.

## Explicitly not required

- **Column widths.** The reference family uses none.
  `DESIGN.md`'s Floor paragraph still lists widths among
  the later arrivals; that wording is owed a correction
  the next time DESIGN is touched for another reason.
- Merged cells, autofilter, conditional formatting
  machinery (`dxfs`), formulas, hyperlinks, data
  validation, multi-row headers, charts, images, defined
  names, protection: none occur in the reference family.
- Shared strings: permitted either way by the reference;
  inline strings are a deliberate thoth position, not a
  gap.

## Open measurement items

- Blank-heavy numeric retention: with witnessed string
  sizing landed (cycle 020), the all-null recipe still
  retains about 1.93× its bytes; whether the hint learns
  blank-heavy numeric columns is undecided.
- The cycle 017 null-timing discrepancy (cross-binary
  probes disagreeing by about ±10–14%) was never settled;
  a within-binary probe is the owed instrument if null
  numbers are ever to be quoted.
- No scaled spec-family recipe lives in `benches/`: the
  full shape is a stored ~911 MB build per iteration,
  recorded as a scratch probe instead (Post-019). Whether
  a reduced-shape in-tree recipe is worth its gate time is
  undecided.
