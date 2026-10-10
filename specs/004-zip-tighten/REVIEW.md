# REVIEW — Cycle 004: Zip internals to machine tightness

Reviewer, 2026-10-10. Read-only review: no source file was edited;
this file is the reviewer's only write.

## Baseline

- `git status --short` (before writing this file):
  ```
  M TESTING.md
  M benches/fill_write.rs
  M src/xlsx.rs
  M src/zip.rs
  ?? specs/004-zip-tighten/
  ```
  (`specs/004-zip-tighten/` held only `SPEC.md` at review time.)
- `git diff --stat HEAD`: 4 files, 418 insertions, 118 deletions
  (`TESTING.md` +69, `benches/fill_write.rs` +24/−1,
  `src/xlsx.rs` +116/−~13, `src/zip.rs` +327/−~104).
- HEAD `bb86154`; changes left uncommitted. No commit made by reviewer.

## (1) Scope — PASS

Changed files are exactly the scoped set from `SPEC.md`
(`src/zip.rs`, `src/xlsx.rs`, `benches/fill_write.rs`, `TESTING.md`,
plus this `REVIEW.md` by role). Nothing else touched.

- `benches/fill_write.rs` is additive only: the diff adds the `crc`
  group (`crc32_iter`, `crc32_4k`, `crc32_1m`) and extends the
  `criterion_group!` line; no existing bench altered.
- No ingress API change: `ColumnData`, `Column`, `XlsxError`/
  `WriteError` shapes and `build_xlsx`/`write_xlsx_file` signatures
  are unchanged in the diff.
- No deflate, cell/emitter, run-off, or `writev` changes.

## (2) Requirements

### R1 — `crc32` tight and vectorized — PASS

- `src/zip.rs:96-107`: instruction spelling under
  `cfg(all(target_arch = "aarch64", target_feature = "crc"))` using
  `__crc32d`/`__crc32b`; `src/zip.rs:114-133`: slicing-by-8 portable
  spelling elsewhere. Compile-time `cfg` only — a repo-wide grep for
  `is_x86_feature_detected|is_aarch64_feature_detected|cfg_if|detect::`
  finds nothing. No runtime dispatch.
- `__crc32cb`-family correctly avoided (Castagnoli polynomial);
  shipped code uses `__crc32b/d` (IEEE).
- IEEE vector `crc32(b"123456789") == 0xCBF4_3926` stays
  (`src/zip.rs:301-304`) and passes in the gate run.
- Fallback correctness is gate-covered on every run: `cfg` selects one
  spelling per target and the `crc_is_ieee` test exercises whichever
  compiled (producer additionally force-compiled the fallback in-tree,
  attested).
- Numbers (`TESTING.md`, machine/profile/commit stated): bitwise 5.8,
  single-table 1.7, slicing-by-8 0.37, instruction 0.083 ns/B;
  permanent `crc` group `crc32_4k` 342 ns, `crc32_1m` 90.7 µs
  (~11.6 GB/s). Internally consistent (342 ns / 4 KiB = 0.084 ns/B;
  90.7 µs / 1 MiB = 0.087 ns/B).

### R2 — `stored_archive_len` as single-expression arithmetic — PASS

- `src/zip.rs:148-154`: one `iter().map(entry_sizes).sum::<u64>() + 22`
  expression; no tuple copies, no statement-per-step loop. Shared
  `entry_sizes` helper (`src/zip.rs:139-141`) also feeds the running
  total in `add` (`src/zip.rs:203`), so the two cannot diverge.
- `finish` no longer walks entries to size itself: `src/zip.rs:220`
  sizes the buffer from the running total (`22 + self.sizes`).

### R3 — Validation at the ingress boundary — PASS

- `stored_archive_fits` is gone from the zip module (grep confirms);
  `archive_fits` lives in `src/xlsx.rs:186-205`, called from
  `build_xlsx` (`src/xlsx.rs:309`) before any assembly byte.
- Constants hoisted to module scope: `U32_MAX`/`U16_MAX`
  (`src/xlsx.rs:163-164`).
- Name shape/uniqueness pinned by `debug_assert`s on the fixed literal
  set in `build_xlsx` (`src/xlsx.rs:319-326`); zip trusts them outright.
- All 5 `archive_fits` unit tests moved with the check and pass
  (in the 11-test gate run).

### R4 — `ZipWriter` assumes valid input — PASS

- `ZipWriter::new(entry_count)` sizes once with `with_capacity`
  (`src/zip.rs:182-187`); `add` carries only `debug_assert!`s
  (relative name, construction-count bound, uniqueness,
  `src/zip.rs:193-201`) — grep confirms no `assert!`/`panic!`/
  `unwrap()`/`expect()` on the non-test build path of `src/zip.rs`.
- `push_u16`/`push_u32` stay named helpers; per-push reserve branches
  retained (safe-`Vec` cost, no `unsafe` added).

### R5 — Byte-identity lock — PASS

- `golden_small_archive_is_byte_identical` (`src/zip.rs:315-339`)
  embeds 207 bytes and passes in the gate run.
- Reviewer independently parsed the embedded bytes (not just re-ran the
  test): declared length 207 == 207 literals; both local entries carry
  correct names/data with CRCs matching `zlib.crc32` (`0x3610a686`,
  `0x1226842b`); central directory starts at 81, holds 2 records;
  end record reads count 2/2, central length 104, central offset 81;
  total 207 == `22 + (76+2·5+5) + (76+2·7+4)`. This is a genuine
  well-formed archive, not filler.
- Provenance ("captured before rewriting") is producer-attested; no
  pre-change artifact survives in-tree to diff against. Content and
  lock behavior are verified; provenance is not independently
  re-verifiable.
- Round-trip suite green: gate ran 11 unit + 1 integration test, all
  pass (see (5)).

### R6 — Numbers recorded — PASS with note

- `TESTING.md` Post-004 section records machine (Darwin 25.6.0 arm64),
  profile, base commit `bb86154` + uncommitted 004 work, pre/post
  criterion means, and deltas: header_only −9%, wide_null −17%,
  tall_null_1000 −58%, tall_null_20000 −76%, tall_null_200000 −78%.
  Percentages recompute exactly from the stated means
  (e.g. 80.98 ms → 18.00 ms = −77.8%).
- Output-bytes-unchanged claim is pinned by the golden test plus the
  per-example verbatim-embed checks (cited in `TESTING.md`, tests green).
- Note: the reviewer did not re-run the criterion benches (cost); the
  deltas are producer-recorded and accepted on internal consistency
  plus the stated machine/profile/commit. The new `crc` group exists,
  compiles (`clippy --all-targets` green), and matches the recorded
  figures dimensionally.

## (3) Cast-to-check mapping (every narrowing cast in `finish`) — COMPLETE

All casts in `src/zip.rs:215-282`, each with its covering ingress check
in `src/xlsx.rs:186-205` (mapping also stated beside each cast and in
the `archive_fits` doc comment):

| Cast (`src/zip.rs`) | Covering ingress check (`src/xlsx.rs`) |
|---|---|
| `usize::try_from(22 + self.sizes)` total buffer size (`:220`) | `archive_fits` refuses past ceilings, so the total fits `usize` on every supported target; saturation would abort loudly in `with_capacity`, never truncate (commented) |
| `out.len() as u32` local offsets (`:225`) | running-offset check `offset > U32_MAX` refuses *before* accumulating each entry, so each entry's start offset fits |
| `e.data.len() as u32` ×2 per entry (`:234-235`, `:255-256`) | `data_len > U32_MAX` refuse (`:195`) |
| `e.name.len() as u16` ×2 per entry (`:237`, `:258`) | `name_len > U16_MAX` refuse (`:195`) |
| `out.len() as u32` central start (`:244`) | final `offset <= U32_MAX` (`:204`); the value is the post-loop running offset |
| `out.len() as u32 - central_start` central length (`:271`) | final `central_len <= U32_MAX` (`:204`); subtraction cannot underflow (`central_start` is an earlier length of the same buffer, commented) |
| `self.entries.len() as u16` ×2 (`:276-277`) | `sizes.len() as u64 > U16_MAX` refuse (`:187`) |

Observation (not a defect): the new check no longer bounds the grand
total at `u32::MAX` — only each part. That is sound: no zip32 field
holds the total, so an archive whose parts each fit is valid; the new
spelling is more permissive than a total-bound exactly where
permissiveness is correct.

## (4) Assembler citations; refutations recorded — PASS

`TESTING.md` carries per-requirement asm notes (source:
`cargo rustc --release -- --emit asm`, same machine, pre vs post), and
`src/zip.rs:13-19` summarizes the post-change shape in-code. The
reviewer re-emitted release asm and confirms the load-bearing claims:

- R1: `crc32` compiles to a `crc32x` word loop plus unrolled `crc32b`
  tail (16 `crc32x`/`crc32b` insns in the lib asm; body inspected:
  `ldr`/`crc32x` loop, compare-chained `ldrb`/`crc32b` tail, no calls,
  no table traffic). The SPEC suspicion is recorded as half-refuted in
  `TESTING.md`: LLVM had already auto-vectorized the bitwise loop
  (NEON bit-sliced), but the instruction spelling is still 69× fewer
  ns/B. Two scratch rejections (`__crc32cb` wrong polynomial;
  interleaved table build reads unfilled row 0) are recorded with
  causes. Nothing silently dropped.
- R2: `finish` sizes from the running total (no entry-walk sizing
  loop in source); `TESTING.md` records the refinement that the two
  `len` spellings are at dynamic parity (~10 hot insns/entry under
  `overflow-checks = true`) and the win is structural. Recorded, not
  dropped.
- R4: `finish`'s `bl` targets in the re-emitted asm are only
  alloc/dealloc/unwind/overflow-`panic_const`/capacity-error paths —
  no `push_u16`/`push_u32` calls, no `memcmp`, no `panic_fmt`
  anywhere in the lib asm (zero matches). The push-helper suspicion is
  recorded as refuted (no `bl` before either) with the reason for
  keeping them (width on the line) and for keeping per-push reserve
  branches (removal needs `unsafe`, no measurement justifies it).
- Pre-change sides of the R1/R2 asm comparisons are
  producer-attested; the reviewer verified the post-change side only.

## (5) Gate — PASS (reviewer-ran)

`export CARGO_HOME=/tmp/cargo-home; sh check.sh` run by the reviewer:
`cargo fmt --check`, `cargo test` (11 unit + 1 integration + 0 doc,
all pass), `cargo clippy --all-targets -D warnings`, `cargo doc` —
ends `all gates green`, exit 0.

## (6) TESTING.md deltas vs recorded runs — CONSISTENT (see R6 note)

Deltas recompute from the stated means; machine, profile, and base
commit are stated; pre-run agreement (~2%) is stated. Not
independently re-run.

## (7) Golden test lock — GENUINE (see R5)

Passes in gate; embedded bytes independently verified as a
well-formed archive with correct CRCs, offsets, and total.

## Findings

1. No defects found. Scope, casts, asm claims, gate, and golden bytes
   all check out; both SPEC refutations are recorded as refuted with
   reasons.
2. Two limits of review evidence (both stated above, neither blocking):
   golden-byte provenance is producer-attested; bench deltas were
   consistency-checked, not re-run.
3. `archive_fits` deliberately permits grand totals past `u32::MAX`
   when every part fits — sound, and worth keeping in mind if a future
   consumer assumes a total bound.

## Verdict per acceptance item

- Gate green (`cargo fmt` clean, `sh check.sh` with
  `CARGO_HOME=/tmp/cargo-home`): **PASS** (reviewer-ran).
- Golden byte-identity test plus full round-trip green: **PASS**.
- `TESTING.md` holds the `floor`/`crc` deltas and assembler notes,
  all from recorded runs: **PASS** (consistency-verified; re-run
  attested, see R6 note).
- Reviewer verdict per item with cast-to-check mapping and assembler
  citations: **this document**; mapping table in (3), citations in (4).
- Applicator reasoning in `DECISION.md`: out of reviewer scope.
- Changes left uncommitted: **PASS**.
