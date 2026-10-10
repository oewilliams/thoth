# Thoth design (locked)

Goal: the fastest Rust `xlsx` writer we can build within a small,
opinionated scope.

## Scope

- Output is a file on disk. Nothing else is a deliverable.
- Cells are primitives only: string, int, float, bool. Dates are not
  handled on our side; they ride through as strings or numbers for Excel
  to parse. Nothing else is permitted for now.
- Cells may be bold, coloured, or both, from a small fixed palette.
  No conditional formatting (a caller can precompute it before filling
  the template). Inline strings; no shared-string hash for the MVP.
- Empty cells, rows, and columns must be possible.
- A CLI is not needed for the MVP. The deliverable is a library.

## Floor

- v0 is the stored-zip floor: `memcpy` plus decimal formatting, frozen
  static parts, two cell styles. Deflate, widths, freeze panes, and
  number formats enter later, each measured against this spelling as
  oracle.
- Opinionated ceiling: sheets past the ~8GB data floor of current
  compute are out of scope. No lazy or backpressured streaming. Line
  everything up and move to disk in as few writes as possible. A
  benchmark showing a faster way overrules this.
- Sealed writes stay: an existing destination is refused, bytes land in
  `.tmp.thoth.*` beside it and rename into place. Crash debris is left
  for a human, never silently cleared.

## Ingress

Delos is the model: data is the imposition of types on memory. The
schema is declared, never sniffed. Lengths are checked once at the
boundary (rectangularity), never per cell.

- Ingress is borrowed columnar slices (`&[T]`): one pointer plus one
  length over contiguous memory. This already removes per-cell pointer
  chasing; fixed `[T; N]` has the same memory shape.
- Compile-time lengths for bulk data are rejected: they push allocation
  upstream, force recompiles per size, risk stack blowup at Excel scale,
  and buy only bounds-check elimination and pre-sizing next to the real
  costs (formatting, escaping, CRC, writes).
- Output buffers are sized once: from const lengths for static sheets,
  from runtime `len()` for dynamic sheets.

## Templates

Nothing is written without a template, and the template is the contract.
Its exact Rust shape (types, macros) is deliberately not locked yet.

- Two template impls are allowed, and a caller exists that needs both:
  statically allocated fixed-size workbooks and dynamic variable-row
  workbooks.
- Monomorphization is the only allowed branch. Static and dynamic share
  the same monomorphic cell emitters; they differ only at the template
  impl, so the hot loop never branches on shape.
- Column-major borrowed ingress is the default. Row-major is deferred
  until a caller shows its fill pattern needs it.
- Strings are the one exception to static allocation. A length cap is
  validation, not an allocation strategy.
