# Cycle 018 — Applicator decision

Written before any applicator code change, after reading SPEC.md,
REVIEW.md, STYLE.md, DESIGN.md, TESTING.md and the producer diff.

Reviewer verdict was SHIP: 0 blockers, 0 majors, 2 minors (M1, M2).
No disagreement with the reviewer; both minors are accepted.

## Will change

- **M1 (accept, fix):** Non-identifier style tokens such as
  `i64 [123] "t"` currently fail with `error: no rules expected '123'`
  instead of the R5 "column style grammar" wording, because the three
  entry macros (`columns!`, `static_sheet!`, `dynamic_sheet!`) accept
  only `$style:ident` inside the brackets. Fix: capture the bracket
  contents as token trees (`$($style:tt)*`) in the entry macros and
  forward them verbatim, so the helper catch-alls and
  `__thoth_style`'s catch-all refuse every non-grammar spelling with
  the grammar message. Valid spellings expand exactly as before, so
  no emitted byte changes.
- **M2 (accept, fix):** `tests/styles.rs` asserts only 6 of the 14
  stored `cellXfs` entries. Fix: assert the full 14-entry table, in
  order, in the stored-part test. Test-only change; no emitted byte
  changes.
- **DESIGN.md Scope (SPEC assigns this to the applicator):** the
  Scope bullet already promises bold/coloured cells, but does not
  state the as-shipped shape. Update it to record that style is
  per column (bold and/or one fixed-palette fill colour), applies
  to data cells, and the header keeps its bold-only spelling.

## Will not change (rejected / no action)

- **Dual index spelling** (`Style::index` arithmetic beside the
  `style_fragment` literal table): reviewer observation, no action
  required. The two spellings are pinned against each other by the
  arithmetic test and the emitter sweep over indices 1..=13; a
  palette change is a spec cycle that must edit both, and merging
  them would reformat the fragment per cell or complicate the hot
  path for no measured gain. Rejected as out of scope for this pass.
- **`sheet_cap` over-reserve on bold-only columns** (7 bytes priced,
  6 spelled): reviewer observation, correct as specced — the cap is
  a proof, never the reservation; `sheet_hint` prices the exact
  fragment. No change.
- **TESTING.md:** the Post-018 entry is already present and was
  verified by the reviewer (machine, profile, commit, agreeing
  pairs, styled-vs-twin pricing). No edit needed; the applicator
  changes above alter no emitted bytes and no bench recipe, so the
  recorded numbers stand.

## Verification plan

- `sh check.sh` green after the changes.
- Re-verify unstyled byte-identity vs HEAD independently: same
  unstyled recipe built against the working tree and against a
  `git archive HEAD` extraction; sheet part must be byte-identical
  and archive growth exactly the 1527 styles-part bytes.
- Confirm `[123]` now refuses with the grammar wording in a scratch
  crate, and that the accepted spellings still compile.
