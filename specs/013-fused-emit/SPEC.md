# Cycle 013 — fused emit (sheet bytes into the archive, CRC on the way)

SUPERSEDED by cycle 014 (arena) and will not run: the arena
prototype showed the two fusions pay only as one layout change,
judged on layout and memory rather than a per-fusion speed gate.
This file is kept as the record of what was proposed.

Phase 3 of 3 (011 multi-sheet → 012 phase bench → 013 fused emit).
Scope is gated by 012's R3 verdict: this spec describes both
fusions, and only the fusions 012 marked `go` (≥ 15% of end-to-end
on agreeing reads) are in scope when the cycle starts. A fusion
012 marked `no-go` is out of scope even though it is described
here; the producer's first act is to copy 012's verdicts into the
work order, and the applicator records any shrink in
`DECISION.md`. If 012 marked both `no-go`, this cycle does not
run and the emitter itself becomes the next spec instead.

Depends on 011 (multi-sheet API) and 012 (phase table) applicated
and green.

## Background

At 011 the build path touches a sheet's expanded bytes four times:
emission formats cells into a per-sheet `String`
(`src/xlsx.rs:236-301`), `ZipWriter::add` re-reads the sheet to
CRC it (`src/zip.rs:192-210`), `ZipWriter::finish` copies the
sheet into the archive buffer (`src/zip.rs:215-282`), and
`fs::write` copies the archive to the kernel. The workbook parts
add the same pattern in miniature (three fragment-cat `Vec<u8>`
staged, checksummed, copied — negligible bytes, same passes).

The micro-opt register this cycle must match is the one already
shipped in these two files, cited here as the standard, not as
inspiration:

- `src/zip.rs` cycle 004: the CRC is the hardware spelling
  (`crc32x` word loop, 0.083 ns/B, `src/zip.rs:96-107`) with the
  slicing-by-8 fallback kept honest beside it; `finish` sizes its
  buffer once from a running total and never walks the entries
  twice; `push_u16`/`push_u32` leave no `bl` on the push paths
  (checked in the assembler, not assumed); `add` carries no
  duplicate scan and no assert formatting in release.
- `src/xlsx.rs` cycle 005: `sheet_xml` allocates once from the
  `sheet_cap` upper bound with `sheet_cap_holds` as the
  no-realloc proof (`src/xlsx.rs:209-231`); row digits are
  formatted once per row into a reused buffer and shared by the
  row's cells (`src/xlsx.rs:268-275`); the five static copies
  were measured (117 ns per build) before anyone was allowed to
  call them cheap.
- `src/emit.rs`: digits are written from the compile-time
  two-digit table with no allocation (`push_magnitude`,
  `src/emit.rs:62-99`), and cycle 006/007 settled the `f64`
  write-through spelling by byte-identity plus bench, tie keeping
  the incumbent.

Concretely, "types are impositions on bytes" earns its place in
this cycle at exactly two joints and nowhere else. First, the ZIP
local header is a fixed 30-byte layout: emitting a sheet directly
into the archive means reserving those 30 bytes plus the name,
emitting over them, and patching the CRC and size fields at their
fixed offsets afterwards — the header is an imposition on a known
byte range, backpatching is its natural spelling, and no data
descriptor or seek is needed because the archive is a memory
buffer on a sealed in-memory build. Second, the CRC becomes a
running state folded per emitted row (the row's bytes are already
in cache; the state is one `u32`), spelled as an update function
beside `crc32` sharing the same instruction loop. Everything else
stays what it already is: a cursor appending fragments. No
general byte-layout framework, no sink trait hierarchy, no new
public abstraction — a generic layer that removes no pass fails
STYLE rule 1 and does not enter.

Error semantics are preserved exactly, and this is the reason the
fusion targets the in-memory archive rather than the file: a data
refusal (`Infinite`, `Ragged`) still aborts before any file
exists, sealed-write debris behaviour is unchanged, and
`sheet_xml` remains as the standalone oracle spelling
(emit-into-a-fresh-buffer) that the verbatim-embed and round-trip
tests check the fused path against.

Binding references: `STYLE.md` rules 1, 4, 5, 7, 8, 9;
`DESIGN.md` Floor (buffer sized once, "line everything up";
"a benchmark showing a faster way overrules this"), Ingress;
`TESTING.md` protocol and spell-agreement rule.

## Requirements (each item testable)

- **R1 — Emitters write bytes, digits write bytes.** The cell
  emitters in `src/emit.rs` take `&mut Vec<u8>` instead of
  `&mut String` (public signature change, recorded as such;
  `emit` is this crate's own module at 0.0.1). Tag fragments are
  `extend_from_slice` of byte constants; `push_i64`/`push_usize`
  write digit bytes directly from `DEC_PAIRS` into the buffer —
  the per-digit `char` loop in `push_magnitude`
  (`src/emit.rs:95-98`) is gone. Header/name escaping keeps its
  current byte output (its input is `&str`; per-char work there
  is header-scale and stays). The `f64` value spelling keeps its
  exact bytes: write-through into the byte buffer via a stack
  scratch (spelling producer's — `fmt::Write` over a small
  fixed buffer, or the cycle-006 backend if it already spells
  bytes — reviewer confirms byte-identity over the cycle-007
  probe set: specials, both zeros, min/max subnormal boundary,
  quarter-steps, and the 301-digit positional extremes). Test:
  emit unit tests green with byte-identical expectations
  (`null_is_blank`, `int_spelling_matches` adapted to bytes,
  `letters_roll_over_z`); a scratch probe over the cycle-007
  value set shows zero `f64` byte mismatches vs HEAD.
- **R2 — Sheets emit into the archive; local headers backpatched.**
  On the `build_workbook` path, each sheet's local header
  (30 bytes + `xl/worksheets/sheet{index}.xml` name) is written
  into the archive buffer with zeroed CRC/size fields, the sheet
  is emitted directly after it by the same emitter code
  `sheet_xml` uses, and the CRC (R3) and compressed/uncompressed
  sizes (equal, stored) are patched at their fixed header
  offsets (CRC at +14, sizes at +18/+22 from the header start —
  producer confirms against `src/zip.rs:226-240`, reviewer
  checks the patch offsets line by line). No per-sheet `Vec<u8>`
  exists on this path; `sheet_xml` remains public and becomes
  emit-into-a-fresh-`Vec<u8>`, same bytes, kept as oracle. The
  generated workbook parts (011 fragment cats) are written into
  the archive the same way — fragments appended in place, CRC
  folded over the emitted range — instead of being staged as
  separate `Vec<u8>`s; `RELS`/`STYLES` likewise enter by copying
  the constant bytes once, directly into the archive (their one
  unavoidable copy is the assembly copy itself). One archive
  allocation: capacity reserved once from the sum of the parts'
  known lengths plus each sheet's `sheet_cap` upper bound plus
  header/central allowances (the `sheet_cap_holds` over-reserve
  is already the shipped sizing philosophy — final length is the
  emitted length, capacity is not length, `fs::write` writes the
  length). The central directory is appended after the last
  entry from the per-entry metadata (name, CRC, size, local
  offset) collected during emission; end record as today.
  `ZipWriter` is reworked to this shape or retired in favour of
  it — one archive builder, never two (producer's shape,
  reviewer confirms no second assembly path survives). Test:
  round-trip, verbatim-embed (`sheet_xml` bytes appear verbatim
  in the archive, now by construction — the test still runs and
  still passes), `golden_small_archive_is_byte_identical`, pins
  and cat-identity green unmodified; archive bytes for every
  store example and the 011 3-sheet example identical to the
  011 output (probe comparing against 011-built files, byte for
  byte, named in `REVIEW.md`).
- **R3 — CRC folds during emission, per row.** `src/zip.rs`
  gains an update spelling beside `crc32` (state in, chunk,
  state out; the shipped instruction loop and slicing-by-8
  fallback both expose it; chunked update over any split equals
  the one-shot `crc32` of the concatenation, pinned by unit
  test including empty chunks and the sheet-like byte patterns).
  On the fused path the sheet's CRC is updated once per emitted
  row over that row's byte range in the archive buffer — never
  over the whole sheet after emission, never per push fragment
  (the fragment count is the per-cell skeleton count, the row is
  the granularity cycle 005 already established for shared
  work). No `crc32(&sheet)` call survives on the build path
  (reviewer greps and lists the surviving call sites: small-part
  CRCs over just-emitted ranges and tests are legitimate,
  a whole-sheet call is not). Test: chunked-equality unit tests
  green; reviewer call-site list in `REVIEW.md`; assembler spot
  check that the update loop is the instruction spelling with no
  per-row call into a non-inlined CRC body the bench did not
  approve (the `push_u16` precedent: checked, not assumed).
- **R4 — Zip32 ceiling with fused lengths.** The ceiling check
  keeps its exactness: `archive_fits` continues to run, now over
  actual emitted lengths where they are known before the cast
  that needs them — each local offset is checked against the
  `u32` bound before its backpatch write (checked arithmetic;
  the running-offset refusal is `TooLarge` with the projected
  bytes spelled as today, upper-bound projection acceptable only
  where the actual is not yet emitted and the refusal is
  therefore conservative in the safe direction — each such site
  named in `REVIEW.md` with why a fitting archive cannot be
  refused there, or the site uses the actual length). The
  cast-for-cast comment block is rewritten to the fused path and
  the reviewer re-derives it cast by cast. Existing
  `archive_fits` unit tests green; the 011 many-sheet test green.
  Test: `sh check.sh` green; reviewer cast list in `REVIEW.md`.
- **R5 — Measured win, or the fusion does not ship.** Re-take the
  012 phase recipes end-to-end (`fill_write`, `cells`, `floor`,
  the 011 3-sheet example) per protocol on agreeing reads. Each
  shipped fusion must show its phase's share actually removed:
  end-to-end improvement at 200k consistent with 012's measured
  share (within the share's range) on at least the two recipes
  that justified the `go`. A fusion that lands within noise is
  reverted and the revert recorded — prettier code at the same
  speed is not a result (STYLE rule 9). The Post-013 entry holds
  both baselines (012 phase table, Post-011 end-to-end), machine,
  profile, commit, box state, and the peak-memory probe from 012
  R4 re-run to show the double residency gone. Test: Post-013
  entry present with the arithmetic tying improvement to the
  012 shares.

## Scoped files

`specs/013-fused-emit/SPEC.md` (this file) now; on `go`:
`src/emit.rs` (byte sink, digit writer, `f64` spelling), `src/zip.rs`
(CRC update spelling; archive builder rework or retirement),
`src/xlsx.rs` (fused `build_workbook`, backpatch, ceiling checks,
`sheet_xml` as oracle wrapper), `TESTING.md` (Post-013 entry),
`specs/013-fused-emit/` records by role. Untouched:
`src/static_parts.rs` (fragment constants as 011 left them),
`src/contract.rs`, `tests/` (byte-identity is the point — a test
that needs editing is a finding, not an edit), `benches/`
(012's phase group is the measurement), `examples/`,
`Cargo.toml`, `Cargo.lock`, `STYLE.md`, `DESIGN.md` (the Floor
wording is updated only if the applicated result changes what
the floor is, and then by the applicator with the reason in
`DECISION.md`). No new dependencies. Probes in `/tmp` scratch.

## Acceptance

- Gate green (`cargo fmt --check`, `sh check.sh`).
- Archive bytes byte-identical to 011 output on every recipe;
  golden, pins, verbatim-embed green unmodified.
- No whole-sheet CRC call and no per-sheet staging `Vec` on the
  build path (reviewer-listed).
- End-to-end win at 200k matching 012's indicted share, on
  agreeing reads; peak-memory probe shows one residency.
- Reviewer verdict per item in `REVIEW.md`; applicator reasoning
  in `DECISION.md`; everything left uncommitted.

## Boundaries (not this cycle)

- Only the fusions 012 marked `go`. Anything else — emitter
  respelling beyond the byte sink R1 requires, parallelism,
  deflate, direct-to-file streaming, data descriptors, seek-back
  patching of a file — is a later spec with its own numbers.
- No public API change beyond the `emit` sink signature in R1
  and whatever the `ZipWriter` rework forces internally;
  `build_workbook`/`write_workbook_file`/`sheet_xml` signatures
  and all error messages stay as 011 left them.
- No `unsafe` without a measurement naming the nanoseconds it
  buys (cycle 004 precedent: the safe-`Vec` push cost was
  measured and `unsafe` was refused).
- The oracle stays: `sheet_xml`'s standalone spelling is
  permanent, and the fused path is tested against it, never the
  other way round (STYLE rule 7).

## Workflow (on `go`)

Producer, reviewer, applicator as three independent agents, one
role each, per `specs/README.md`: the producer gets this SPEC plus
`STYLE.md`, `DESIGN.md`, `TESTING.md`, the applicated 012
`TESTING.md` entry with its R3 verdicts, and nothing else; the
reviewer reads code and spec, builds, runs probes and tests, and
creates only `REVIEW.md` (the tree byte-identical before and
after outside scoped files, verified by diff); the applicator
applies required changes and writes `DECISION.md`, leaving
everything uncommitted. No agent reviews its own work.
