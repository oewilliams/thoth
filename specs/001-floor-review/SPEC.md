# Cycle 001 — Floor review hardening: SPEC

Lock the v0 floor at contract level: emitted bytes, error taxonomy, and
the seams later stages depend on. No cycle-level optimization; anything
the future CRC-fusing step will delete is explicitly out of scope.

## Scope (read-only except as required below)

- `src/zip.rs` — stored-only assembly, reserve math, `as u32` casts.
- `src/xlsx.rs` — capacity sizing, `build_xlsx`, `write_xlsx_file`,
  `XlsxError`, sealed-move semantics.
- `src/emit.rs`, `src/contract.rs` — read for context; do not change.
- `tests/roundtrip.rs`, `tests/common/` — read for context; the only
  test change allowed is additive unit tests for the new guard.

## Facts the spec depends on

- Archive layout reserves are exact: 30 bytes per local header, 46 per
  central record, 22 end record. Keep them exact.
- Lengths and offsets narrow to `u32` with `as` casts, which truncate
  silently past 4GB even with overflow checks on. There is no zip64
  support, so 4GB is a hard format ceiling.
- Public shape to preserve: borrowed columns in, one owned buffer out,
  sealed temp-file plus rename to disk. Pre-sized buffers stay.
- Error style: lowercase messages stating the limit, e.g. the existing
  row/column limit errors. `Display` plus `Error` impls required.

## Requirements

1. **4GB boundary guard.** Refuse loudly before assembling anything
   when the finished archive cannot fit the 32-bit fields. Coverage
   must include every length/offset cast on the path: part data
   lengths, local header offsets, central directory offsets and length,
   end record counts.
2. **Check placement.** Fail fast in the build path before formatting
   or assembly work, not after. Exact function placement is the
   producer's choice; the reviewer verifies every cast is covered.
3. **New error variant.** Surface through the existing public error
   type in matching style (suggested: `TooLarge { bytes: usize }`
   stating the 4GB zip ceiling). Additive variant only.
4. **Pure predicate, tested at the boundary.** Isolate the fit decision
   in a pure function of sizes (no allocation) and unit-test it at
   `u32::MAX - 1`, `u32::MAX`, and `u32::MAX + 1` without allocating
   gigabytes. Tests must pass in debug and release profiles.
5. **Byte oracle lock.** Zero byte changes for all in-range inputs.
   Proof is the existing round-trip tests plus the per-example verbatim
   checks, all green, unmodified in expectation.
6. **Docs.** Doc comments on every new public-or-crate item; the doc
   gate denies warnings.

## Non-goals (do not touch)

CRC algorithm or its pass structure, staging fusion, number or string
formatting, column letters, template types, benchmark files or scales,
test expectations, new dependencies, any cycle-level tuning.

## Acceptance

- `sh check.sh` green (note: in this sandbox run all cargo commands
  with `CARGO_HOME=/tmp/cargo-home` exported; that workaround is
  environment, not code).
- New boundary unit tests pass in both profiles.
- `cargo bench --bench fill_write floor` within noise of the baseline
  recorded in `TESTING.md`.
- Diff touches only `src/zip.rs`, `src/xlsx.rs`, and additive unit
  tests. Anything else sends the cycle back.
