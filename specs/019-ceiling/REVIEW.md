# Cycle 019 — review

Reviewer, independent of the producer. Tree reviewed: HEAD
`3c1c266` plus uncommitted 018 work plus uncommitted 019 work.
Nothing committed by this review. The only file this review
wrote is this `REVIEW.md`; the source tree (every other file,
tracked and untracked, excluding `target/`) is byte-identical
before and after the review — verified by a full-tree SHA-256
manifest taken before any review work and re-checked after
this file was written, plus `git status`/`git diff HEAD`
hashes (method and result in Evidence below).

## Verdict

Ship the 019 change. No blocker and no major finding: every
SPEC requirement R1–R6 holds on inspected evidence. Two minor
documentation findings and one nit follow; none changes a
byte, an error value at the SPEC-pinned site, or a verdict.

## Findings by severity

### Blocker

None.

### Major

None.

### Minor

- **M1 — `TooLarge.bytes` doc comments overgeneralise past
  the sheet-refusal site.** The variant doc
  ([src/xlsx.rs](/Users/oliverwilliams/Documents/thoth/src/xlsx.rs:66))
  calls `bytes` "its projected finished length, computed from
  witnessed lengths", and `checked_u32`'s doc
  ([src/zip.rs](/Users/oliverwilliams/Documents/thoth/src/zip.rs:174))
  calls it `stored_archive_len` "over the actual entries
  witnessed up to and including the refusing entry". Both
  sentences are exact only at the sheet patch site
  ([src/xlsx.rs](/Users/oliverwilliams/Documents/thoth/src/xlsx.rs:402):
  `projected + entry_sizes(len, name.len())` over the entries
  so far, which this review traced entry by entry to
  `stored_archive_len` over that prefix). At the other sites
  the value differs in kind, not in correctness:
  (a) the pre-emission refusal
  ([src/xlsx.rs](/Users/oliverwilliams/Documents/thoth/src/xlsx.rs:498))
  prices sheets at `(0, name_len)`
  ([src/xlsx.rs](/Users/oliverwilliams/Documents/thoth/src/xlsx.rs:485)),
  so its `bytes` excludes all sheet payload and is a partial
  figure, not the projected finished length the variant doc's
  first sentence promises; (b) the `finish_archive` sites pass
  `projected` over *all* witnessed entries
  ([src/xlsx.rs](/Users/oliverwilliams/Documents/thoth/src/xlsx.rs:421)),
  so a central-start/offset/length refusal reports the full
  finished length, not the "up to and including" prefix the
  `checked_u32` doc states. The SPEC pins only the
  sheet-refusal semantics (lower bound), and that site is
  exact; the error variant, fields, and message shape are
  unchanged (variant
  [src/xlsx.rs](/Users/oliverwilliams/Documents/thoth/src/xlsx.rs:72),
  message
  [src/xlsx.rs](/Users/oliverwilliams/Documents/thoth/src/xlsx.rs:164)).
  Fix is doc wording only: scope the finished-length sentence
  per refusal site. No other minor finding.

### Nit / informational

- **N1 — central start is checked before the central records
  are written, not literally at the end-record write.** SPEC
  wording is "checked where they are written";
  `central_start` is narrowed at the top of `finish_archive`
  ([src/xlsx.rs](/Users/oliverwilliams/Documents/thoth/src/xlsx.rs:423))
  and only written later by `write_end_record`
  ([src/xlsx.rs](/Users/oliverwilliams/Documents/thoth/src/xlsx.rs:440)).
  Behaviourally indistinguishable: same witnessed value,
  same `bytes` payload (`projected` over all entries), refusal
  still after emission, partial `out` discarded on error.
- **N2 — the small-part `checked_u32` can never fire.**
  `emit_small_part` narrows through `checked_u32`
  ([src/xlsx.rs](/Users/oliverwilliams/Documents/thoth/src/xlsx.rs:349)),
  but `archive_fits` already refuses any small part past
  `DATA_MAX` before emission, so that refusal is unreachable.
  The uniform spelling is the SPEC's one-spelling rule, not
  dead weight worth removing; recorded so nobody later reads
  it as a reachable path.

## Requirement-by-requirement

- **R1 (spec-sheet shape builds) — pass, independently
  reproduced.** The gated test
  ([tests/ceiling.rs](/Users/oliverwilliams/Documents/thoth/tests/ceiling.rs:15))
  builds 3 sheets × 8,880 rows × 1,298 `f64` columns,
  blank-heavy (`NaN`, one `0.0` column), and asserts the
  relationship it relies on inside the test
  ([tests/ceiling.rs](/Users/oliverwilliams/Documents/thoth/tests/ceiling.rs:45)):
  projection > ceiling ≥ actual. Reviewer probe (scratch
  `/tmp/reviewprobe`, release, `overflow-checks = true`,
  `codegen-units = 1`, this machine): projection
  **12,291,599,478** > ceiling **4,294,967,295** ≥ actual
  **530,611,658** — the exact figures the TESTING Post-019
  entry records. The same probe pointed at the pre-cycle tree
  refuses with `TooLarge { bytes: 12291604988 }`, so the test
  really separates pre/post behaviour. The full-`0.0` shape
  builds at **910,684,538** actual bytes (reviewer single
  build, 593 ms cold; producer best-of-5 533.5 ms — same
  order, no contradiction). Per-sheet cap 4,097,199,826 × 3
  = 12,291,599,478 matches TESTING. Test cost in the gate:
  1.53 s in the debug test profile — sane.
- **R2 (genuine overflow refuses at the cast) — pass.** One
  checked spelling, `checked_u32(len, projected)`
  ([src/zip.rs](/Users/oliverwilliams/Documents/thoth/src/zip.rs:181),
  `u32::try_from`, no allocation), covers every
  sheet-dependent narrowing: sheet length at the local-header
  patch ([src/xlsx.rs](/Users/oliverwilliams/Documents/thoth/src/xlsx.rs:407)),
  central start ([src/xlsx.rs](/Users/oliverwilliams/Documents/thoth/src/xlsx.rs:424)),
  per-entry length and local offset
  ([src/xlsx.rs](/Users/oliverwilliams/Documents/thoth/src/xlsx.rs:428)),
  central length ([src/xlsx.rs](/Users/oliverwilliams/Documents/thoth/src/xlsx.rs:439)).
  Synthetic boundary coverage with no multi-GB allocation:
  `checked_u32_narrows_at_the_u32_boundary`
  ([src/zip.rs](/Users/oliverwilliams/Documents/thoth/src/zip.rs:323):
  at `u32::MAX` narrows, `MAX+1`/`MAX+4096` refuse, `bytes`
  equals the supplied projection) and
  `checked_narrowing_refuses_past_the_u32_boundary`
  ([src/xlsx/tests.rs](/Users/oliverwilliams/Documents/thoth/src/xlsx/tests.rs:62):
  0/1/`MAX−1`/`MAX` narrow, `MAX+1`/`MAX+88000` refuse with
  `bytes = stored_archive_len` over the witnessed triple).
  The pre-existing `archive_fits` synthetic tests (count,
  names, small-part totals, central-over-known) are untouched
  by the 019 diff and pass (archive_fits body itself is
  byte-unchanged; only its doc moved).
- **R3 (no byte changes) — pass, independently reproduced.**
  Reviewer rebuilt the producer's eight dump recipes from
  scratch against both the pre-cycle tree
  (`/tmp/thoth-pre019-tree`) and the current tree
  (`/tmp/reviewprobe`, paths swapped, nothing else changed):
  `cmp` identical on ints200k (10471536), floats200k
  (10871537), null200k (10471598), mixed200k (17049378),
  strings_bools200k (17471636), styled_small (5106, styles
  part included), three_sheet200k (21339645), small (5028).
  The reviewer's fresh dumps also match the producer's
  `/tmp/thoth-pre019-dumps` vs `/tmp/thoth-post019-dumps`
  byte for byte, so the producer artifacts are corroborated,
  not merely re-read. The golden test's expected 207 bytes
  are unchanged, re-spelled through `checked_u32`
  ([src/zip.rs](/Users/oliverwilliams/Documents/thoth/src/zip.rs:357)).
- **R4 (no other verdict moves) — pass.** Boundary validation
  still runs before any emission and before the pre-emission
  ceiling check: `validate_sheet_names`
  ([src/xlsx.rs](/Users/oliverwilliams/Documents/thoth/src/xlsx.rs:454);
  body in `src/workbook_parts.rs:169`, untouched by 019),
  then `validate_template` per sheet
  ([src/xlsx.rs](/Users/oliverwilliams/Documents/thoth/src/xlsx.rs:456);
  empty/kind/row/column/ragged/infinite/text checks in
  `src/contract.rs:424`, untouched by 019), then
  `archive_fits`, then the reservation, then `emit_parts`
  ([src/xlsx.rs](/Users/oliverwilliams/Documents/thoth/src/xlsx.rs:519)).
  The reservation is still the hint (or `ARENA_BLOCK`)
  ([src/xlsx.rs](/Users/oliverwilliams/Documents/thoth/src/xlsx.rs:491),
  [src/xlsx.rs](/Users/oliverwilliams/Documents/thoth/src/xlsx.rs:510)),
  with arithmetic unchanged from the pre-cycle tree, so a
  build that will ultimately refuse reserves exactly what it
  reserved before. Neither estimate can veto: `sheet_cap`'s
  only production consumer is now the oracle's capacity
  (`src/contract.rs:660`) plus the `sheet_cap_holds` proof;
  `build_workbook` no longer imports it.
- **R5 (measured record) — pass with one labelled limit.**
  The Post-019 entry
  ([TESTING.md](/Users/oliverwilliams/Documents/thoth/TESTING.md:1325))
  states machine (Darwin 27.0.0 arm64, Apple M4,
  `RELEASE_ARM64_T8132`), profile, rustc 1.93.1, and the
  pre-cycle tree state (HEAD `3c1c266` plus uncommitted 018);
  records the three-sheet probe with cells (34,578,720),
  actual bytes, best time, MB/s, and peak RSS beside the
  one-sheet reading; shows existing recipes on interleaved
  pre/post reads with no speedup claimed; and records the
  bench call explicitly — no scaled recipe joins
  `benches/fill_write.rs` because a stored ~911 MB build per
  criterion iteration is not a bench. This review verified
  the entry's load-bearing figures directly (projection,
  blank-heavy actual, full-`0.0` actual, byte-identity) but
  did **not** re-run the interleaved timing matrix or the
  RSS processes; those reads stand as producer-measured.
- **R6 (docs catch up) — pass.** DESIGN Floor states the
  witnessed verdict
  ([DESIGN.md](/Users/oliverwilliams/Documents/thoth/DESIGN.md:35)),
  Ingress names the witnessed-length exception
  ([DESIGN.md](/Users/oliverwilliams/Documents/thoth/DESIGN.md:58)),
  Templates removes `sheet_cap` from the ceiling story,
  keeps `archive_fits` for what is known before emission, and
  states neither estimate may veto
  ([DESIGN.md](/Users/oliverwilliams/Documents/thoth/DESIGN.md:99)).
  Cast-cover comments in `src/xlsx.rs`/`src/zip.rs` now name
  the witnessed check per cast (patch, central, end sites;
  the surviving `as u16` covers name their pre-emission
  `archive_fits` refusal on the adjacent line,
  [src/zip.rs](/Users/oliverwilliams/Documents/thoth/src/zip.rs:213),
  [src/zip.rs](/Users/oliverwilliams/Documents/thoth/src/zip.rs:252),
  [src/zip.rs](/Users/oliverwilliams/Documents/thoth/src/zip.rs:276)).

## Prompt checklist

- **No sheet-dependent `as u32` without the checked
  narrowing on the line — confirmed.** `grep 'as u32|as u16'`
  over `src/`: production hits are only the CRC table byte
  cast (`src/zip.rs:73`, a `0..256` const-table index, not a
  length) and the three `as u16` name/count casts above, each
  covered by the pre-emission `archive_fits` refusal named on
  the adjacent comment line. No `as u32` on any length
  survives in `src/xlsx.rs`/`src/zip.rs` production code.
- **`TooLarge.bytes` vs its doc — exact at the pinned site,
  loose elsewhere; see M1.** Error variant, fields, and
  message shape unchanged, as the SPEC requires.
- **Pre-emission checks still exact for count/names/small
  parts — confirmed.** `archive_fits`
  ([src/xlsx.rs](/Users/oliverwilliams/Documents/thoth/src/xlsx.rs:310))
  is fed small parts at actual length and sheets at
  `(0, name_len)`; over those inputs the count, per-name,
  per-part, local-total, and central-over-known checks are
  exact (central footprint depends on names and count only).
  Its arithmetic is unchanged from the pre-cycle tree.
- **Boundary validation still before emission — confirmed**
  (R4 ordering above; 019 diff touches none of it).
- **Byte-identity vs the pre-019 tree — confirmed
  independently** (R3; reviewer-built dumps on both trees,
  producer dumps corroborated).
- **R1 test really has projection over ceiling and actual
  under it — confirmed** (assertions inside the test plus
  reviewer probe numbers; pre-cycle tree refuses the shape).
- **STYLE compliance — confirmed for the 019 hunks.**
  Comments added by 019 are minimal, name the witnessed check
  each cast relies on, and contain no cycle or plan/history
  prose (`grep cycle|019|018|plan` over the touched files
  hits only pre-existing base text: the cycle-004 assembler
  note `src/zip.rs:23` and the cycle-018 file-size tripwire
  `src/contract.rs:6`, both present in the pre-cycle tree).
  Conversions spell the target on the line
  (`u32::try_from`), no new `unwrap` on data (`unwrap_or`
  saturation only, commented as error-payload/reservation
  saturation), imports item-by-item, no new dependency
  (`Cargo.toml`/`Cargo.lock` untouched), scoped files match
  the SPEC list exactly.

## 018 vs 019 separation

Method: `git diff HEAD` (018+019 combined) against
`diff -r /tmp/thoth-pre019-tree .` (019 only; the snapshot
is the 018-complete working tree taken before any 019 edit —
it contains `specs/018-styles/`, `tests/styles.rs`, and the
full styles contract, and its `specs/019-ceiling/SPEC.md` is
byte-identical to the current one).

| File | Belongs to |
|---|---|
| `src/emit.rs`, `src/macros.rs`, `src/lib.rs`, `src/static_parts.rs`, `src/workbook_parts.rs`, `benches/fill_write.rs`, `tests/common/mod.rs`, `tests/multi_sheet.rs`, `tests/primitives.rs`, `tests/template.rs`, `examples/styled.rs`, `tests/styles.rs`, `specs/018-styles/` | 018 only (byte-identical in the pre-cycle snapshot) |
| `tests/ceiling.rs` | 019 only (new file) |
| `src/xlsx.rs` | Shared |
| `src/zip.rs` | Shared |
| `src/contract.rs` | Shared |
| `src/xlsx/tests.rs` | Shared |
| `DESIGN.md`, `TESTING.md` | Shared |

Shared-file attribution: in `src/contract.rs` the 019 hunk
is only the `sheet_cap` doc paragraph
(`src/contract.rs:543`); everything else (`Colour`/`Style`,
styled cap/hint pricing, drivers) is 018. In
`src/xlsx/tests.rs` the 019 hunks are the `checked_u32`
import and `checked_narrowing_refuses_past_the_u32_boundary`;
the styled `sheet_cap_holds` block is 018. In `src/xlsx.rs`
and `src/zip.rs` the 019 hunks are exactly the unified diff
against the pre-cycle snapshot (build verdict, `checked_u32`,
`finish_archive`, running `projected`, cast-cover comments);
the remaining HEAD diff in those files is 018. In
`DESIGN.md`/`TESTING.md` the 019 hunks are the Floor/
Ingress/Templates wording and the Post-019 entry after the
Post-018 entry ([TESTING.md](/Users/oliverwilliams/Documents/thoth/TESTING.md:1240)).

## Evidence

- Gate: `sh check.sh` on the reviewed tree —
  `cargo fmt --check`, `cargo test --locked` (73 tests,
  including the 1.53 s ceiling build), `cargo clippy
  --locked --all-targets -- -D warnings`, `cargo doc
  --locked --no-deps` — output ends "all gates green".
- Independent probes (scratch `/tmp/reviewprobe`, release
  profile as above, Darwin 27.0.0 arm64, rustc 1.93.1):
  byte-identity cmp on eight recipes across both trees;
  ceiling-shape numbers (projection 12,291,599,478, actual
  530,611,658 blank-heavy, 910,684,538 full-`0.0`);
  pre-cycle refusal `TooLarge { bytes: 12291604988 }` on the
  R1 shape. Producer dumps in `/tmp/thoth-pre019-dumps` and
  `/tmp/thoth-post019-dumps` match the reviewer's fresh
  dumps byte for byte.
- Tree integrity: SHA-256 manifest of the full tree
  (excluding `.git/`, `target/`) taken before review work
  and re-verified after writing this file; only
  `specs/019-ceiling/REVIEW.md` differs (added). `git status`
  and `git diff HEAD` hashes recorded with the manifest in
  `/tmp/thoth-review-pre.sha256` (review scratch, not part
  of the deliverable). HEAD still `3c1c266`; nothing
  committed.
