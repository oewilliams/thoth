# Cycle 019 — Applicator decision

Written before any applicator code change, after reading SPEC.md,
REVIEW.md, STYLE.md, DESIGN.md, TESTING.md and the current diff
(HEAD `3c1c266` plus uncommitted 018 and 019 work).

Reviewer verdict was ship: no blocker, no major finding, one minor
(M1) and two nits (N1, N2). No disagreement with the reviewer on
the substance of M1; the nits are not taken as code moves, for
the reasons below.

## Will change

- **M1 (accept, doc wording only):** the `TooLarge.bytes` docs
  overgeneralise past the sheet-refusal site. The variant doc in
  `src/xlsx.rs` and the `checked_u32` doc in `src/zip.rs` will be
  scoped per refusal site, keeping the SPEC-pinned sheet
  semantics exact and unchanged in behaviour:
  - before emission (`archive_fits` refusal): `bytes` is
    `stored_archive_len` over the known sizes alone, sheets
    priced at zero data length, so it excludes sheet payload;
  - at a part patch (sheet, and the same running-total spelling
    in the small-part patch): `bytes` is `stored_archive_len`
    over the actual entries up to and including the refusing
    entry, a lower bound on the finished archive;
  - in `finish_archive` (central start, per-entry length and
    offset, central length): `bytes` is `stored_archive_len`
    over every witnessed entry, the projected finished length.
  The error variant, its fields, and its message do not change,
  and no emitted byte changes.
- **N1 (accept as wording only):** the central-start comment in
  `finish_archive` will stop implying the narrowing sits at the
  end-record write; it will say the witnessed length is narrowed
  before the central records are written and the value is
  written into the end record below. No code moves.

## Will not change (rejected / no action)

- **N1 as a code move (rejected):** relocating the central-start
  narrowing to the end-record write changes nothing observable —
  same witnessed value, same `bytes` payload (`projected` over
  all entries), refusal still after emission with the partial
  `out` discarded on error, as the reviewer notes. Moving it
  would churn the emission order for no behavioural gain.
- **N2 (rejected):** removing the small-part `checked_u32` because
  the pre-emission `archive_fits` check makes it unreachable
  would break the SPEC's one checked-narrowing spelling for
  sheet-dependent and part lengths. The uniform spelling stays;
  unreachability is a consequence of the exact pre-check, not
  dead weight.

Nothing else in the reviewer checklist calls for a change: R1–R6
pass, no sheet-dependent `as u32` survives without the checked
spelling, boundary validation still precedes emission, and the
018/019 hunk separation stands as the reviewer recorded it.

## Verification plan

- `sh check.sh` green after the changes.
- Re-verify byte-identity against the pre-019 tree
  (`/tmp/thoth-pre019-tree`): the applicator changes are doc
  comments only, so the eight recorded dump recipes rebuilt from
  the current tree must still `cmp`-match the pre-019 dumps,
  styles part included.
