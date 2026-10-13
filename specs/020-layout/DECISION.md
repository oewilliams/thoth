# Cycle 020 — Applicator decision

Written before any applicator code change, after reading
`specs/020-layout/SPEC.md`, `specs/020-layout/REVIEW.md`,
`STYLE.md`, `DESIGN.md`, `TESTING.md` (including the Post-020
entry) and the current uncommitted diff against HEAD `a0fddc9`
(`src/contract.rs`, `src/emit.rs`, `src/xlsx.rs`, `TESTING.md`;
the pre-existing `RUNOFF.md` entry excluded, untouched, and
nothing here is committed).

Reviewer verdict was do not commit as-is: one blocker (B1),
one major (M1), three minors. The note below records what
this applicator will change, what it will not, and why,
before any code moves. One evidence gap is named up front:
`REVIEW.md` as stored ends mid-sentence in its
"Required before commit" item 2 ("…with re-measured
capacities if" followed by a literal truncation marker in
the file). The governing intent is unambiguous from the M1
section itself — fix the count, or retract "exact" with
re-measured capacities — so the applicator proceeds on the
M1 section's own text and notes the truncated tail here
rather than guessing its last clause.

No disagreement with the reviewer on the substance of
B1 or M1: both were reproduced from the review's own
evidence before this file was written (the shipped
`escape_into` slice points and the `swar_count_byte`
borrow formula are exactly as the review describes them).
The disagreements below are about remedy shape and
scope only, and they are recorded here, before the edits.

## Will change

- **B1 (accept; blocker): `escape_into` must not slice
  mid-char.** In `src/emit.rs` the clean-word SWAR skip
  (`i += 8` on `!swar_has_escapable`) will also require
  the word to be plain ASCII (`swar_word_is_plain_ascii`,
  the same gate the boundary scan in `src/contract.rs`
  already uses), so every `&text[start..i]` / `text[i..]`
  slice point sits on a char boundary or a text end by
  construction. The dirty-word walk already advances by
  whole chars and does not change. `escape_into_bytes`
  does not change: it slices bytes, never `str`, and the
  reviewer confirmed it unaffected — gating it would be
  churn with no correctness content.
  The doc sentence at the head of `escape_into` ("slice
  points sit on them or on the text ends") will be
  rewritten to state the actual invariant (clean words
  are plain-ASCII, so word ends are boundaries).
  Pinned cases will be added to
  `escape_spelling_is_pinned` covering 2-, 3- and
  4-byte chars straddling an 8-byte word boundary at
  every straddle offset, each with and without an
  escapable in a following word, asserted on both
  sinks — the exact placements the shipped test missed
  because its only multi-byte char sat inside an
  already-dirty word.
- **M1 (accept; major): make the witness exact, keep
  the word "exact".** The alternative the SPEC's own
  text disfavours — retracting "exact" and re-measuring
  capacities around an inflated reservation — is
  rejected: SPEC E1 requires the *exact* escaped length,
  the inflation only ever wastes reservation, and an
  exact borrow-free lane count exists at a few extra
  mask steps on a scan the reviewer verified is not the
  bottleneck. `swar_count_byte` in `src/emit.rs` will
  count zero lanes with the borrow-free spelling (clear
  each lane's high bit, add `0x7F` per lane — no
  inter-lane carry is possible — or-in the original
  high bit, invert): the high bit then marks a zero lane
  exactly, with no borrow out of a true zero lane able
  to flag a following `target ^ 1` lane. `swar_has_byte`
  is unchanged: existence is already sound, since a
  borrow can only originate at a true zero lane, so it
  never invents a hit. The witness test
  `witnessed_escaped_total_is_exact` gains the
  reviewer's counterexample patterns (`&'&'&'&'`,
  `'&'&'&'&`, `&<>"'&<>`, `aaa&'aaa`, expectations by
  per-char spelling) so an over-count regresses the
  gate. With the count exact, the "exact" claims in
  `src/contract.rs`, the test name and the TESTING
  Post-020 entry stand as written; capacities will be
  re-read after the fix and the Post-020 entry corrected
  only if a recorded figure actually moves (the fix can
  only shrink the witness, never grow it, so a capacity
  can only hold or improve).
- **Minor, doc wording (accept):**
  `validate_template_witnessed` docs claim "the total
  is partial" on refusal. `scan_text_cells` accumulates
  across the sheet and the caller discards the total on
  any refusal; the sentence will say discarded/unused,
  not partial. No behaviour change.
- **Minor, tripwire annotations (accept, wording
  only):** the STYLE rule 1 annotations at the head of
  `src/emit.rs` and `src/contract.rs` still cite cycle
  018 as the cycle that passed 700 lines; both files
  grew again in 020. The annotations will own the 020
  growth in the same sentence form. Splitting either
  file is not taken: the breach is pre-existing,
  annotated, and every new 020 function is under the
  100-line tripwire — a split this cycle would be
  scope the SPEC did not ask for.

## Will not change (rejected / no action)

- **No change to `escape_into_bytes` for B1** (see
  above): byte slices cannot split a char, the review
  verified the byte sink matches HEAD byte-for-byte on
  the straddle inputs, and touching it risks the
  verified assembly shape (one `ldr x` per word, no
  call on the clean path) for nothing.
- **No retraction of "exact", no capacity re-derivation
  as a substitute for the M1 fix** (see above). If the
  borrow-free count measurably slowed the boundary
  scan this decision would reverse; it will be checked
  by the gate plus the capacity re-read, and any
  surprise recorded as an amendment below rather than
  silently absorbed.
- **No file split for the 700-line tripwire**, and no
  other reviewer-checklist item calls for code: the
  rejected candidates (E2/E3/E5) leave no code, no
  payload classification past the boundary was found,
  `src/zip.rs` is untouched, and R1 spot checks, the
  E4 assembly claim and the long-string timing were
  independently verified by the reviewer. Those reads
  are relied on, not re-derived; only the items the
  fix touches are re-verified below.

## Verification plan (run after the edits)

- `sh check.sh` green on the final tree (format, tests
  including the new pin/counterexample cases, clippy
  `-D warnings`, docs).
- Byte-identity re-verified, because the reviewer
  flagged it (B1 makes the oracle crash where the
  arena succeeds): a scratch probe against a
  `git archive` HEAD extraction comparing
  `build_workbook` output byte-for-byte on the
  reviewer's straddle inputs through both entry points
  (sheet name via the oracle path, string cells via the
  arena), plus the reviewer's spot recipes
  (escapable-heavy, long strings, multi-byte byte-sink
  set) and the wider Post-020 list as feasible; HEAD
  crashes on the oracle straddle inputs, so identity
  there is judged as: fixed oracle output equals the
  arena/HEAD byte-sink output on the same inputs.
- The exactness of the new count checked against a
  per-char oracle on the reviewer's pattern words and
  a random-word sweep, and the escapable-heavy
  retained-capacity direction re-read on the fixed
  tree.
- `RUNOFF.md` untouched throughout; nothing committed.

## Outcome (applicator's own runs, final tree)

- `cargo fmt` applied, then `sh check.sh` green: format,
  tests (including the new straddle pins and witness
  counterexamples), clippy `-D warnings`, docs —
  "all gates green".
- B1 re-verified: a scratch copy with only the plain-ASCII
  gate removed panics at the flush (`byte index 8 is not
  a char boundary; it is inside '£'`) on
  `abcdefg£hijklm&` through both `sheet_xml` and
  `build_workbook` (sheet name), reproducing the review;
  the fixed tree panics nowhere. Whole-archive/output
  comparison against a `git archive` HEAD extraction
  (`/tmp/verify020`, `/tmp/thoth-head-verify`): arena
  straddle cells, oracle straddle XML, straddling sheet
  name, escapable-heavy 20k, 50k ints, 2,000 long
  strings and the multi-byte set are byte-identical
  (`cmp`) to HEAD on every recipe.
- M1 re-verified: the borrow-free count matches a
  per-byte oracle on 1,000,000 random word/target pairs
  (old formula: 101 mismatches) and exhaustively on all
  two-byte lane pairs. Through the shipped test, the
  old formula witnesses 75 against the exact 44 on
  `&'&'&'&'` (the review's figure, reproduced); the
  fixed tree witnesses 44. Retained capacity on the
  `&'`-saturated recipe moves 1.4299x (old count) to
  1.0328x (fixed); on the review's escapable-heavy
  shape the fixed tree reads 1.0409x, holding the
  recorded Post-020 1.044x/1.026x direction, so no
  TESTING figure is corrected — the fix can only
  shrink the witness, and on the recorded recipes it
  does not move them.
- No disagreement with the reviewer remains open.
  Remaining risk is the one named in the plan above:
  the plain-ASCII gate makes `escape_into` walk words
  holding control bytes (< 0x20) char by char; those
  bytes are refused at the boundary for string cells
  and are vanishingly rare in the header/sheet-name
  texts that reach the oracle, and no timing claim is
  made beyond the reviewer's verified reads. Files
  changed by the applicator: `src/emit.rs`,
  `src/contract.rs`, and this `DECISION.md`;
  `RUNOFF.md` untouched, nothing committed.
