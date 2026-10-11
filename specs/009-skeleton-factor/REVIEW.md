# Cycle 009 review — skeleton-factor SPEC

Reviewer verification, 2026-10-11. No producer diff exists yet (prior
result 1 delivered only `SPEC.md`), so this review verifies each
requirement's premises against the actual code, re-runs the gate, and
rules the SPEC shippable or not. Every cited body was opened and read.
No source file was edited by the reviewer: all tracked bodies
hash-identical before and after (hashes below);
`git status --porcelain` shows only the untracked
`specs/009-skeleton-factor/` directory. `git diff HEAD --stat` is
empty.

## Verdicts

- R1 (single-file layout, no per-kind split): APPROVE.
- R2 (monomorphic loops, no per-cell dispatch): APPROVE, with one
  advisory on the helper shape (§R2).
- R3 (byte-identity, suite green unmodified): APPROVE.
- R4 (bench proof, no regression): APPROVE.
- R5 (comment minimalism): APPROVE.
- The SPEC as a whole: SHIPPABLE. No rejections.

## Evidence per item

### R1 — single-file layout stays

Premises verified. The user ruling (no per-kind module split until
the i32 cycle) is honored exactly: the helper lives in `src/emit.rs`
as a private item beside `i64_cell` / `f64_cell`, no new module, no
per-kind file, no `pub` widening. Scoped files (§Scoped files)
confine the applicator to `src/emit.rs` plus the `TESTING.md`
Post-009 entry, with `src/xlsx.rs`, `src/contract.rs`, `src/zip.rs`,
`src/static_parts.rs`, `tests/`, `benches/`, `examples/`,
`STYLE.md`, `DESIGN.md` explicitly untouched. The test (no new `.rs`
file in `git status`, helper non-`pub`, exact public signatures kept)
is sufficient and directly checkable.

### R2 — monomorphic loops preserved

Premises verified against the cited bodies, all read in full:

- `src/emit.rs:105-119` (`i64_cell`) and `:125-142` (`f64_cell`)
  confirm the Background's spelling claim: both arms repeat the
  `<c r="` + letters + row open, the `"... />` blank close
  (`:107-110`, `:127-130`), and the `"><v>` … `</v></c>` value
  close (`:112-117`, `:132-141`). Value spellings differ per kind
  (`push_i64` at `:116` vs `write!(out, "{v}")` at `:136`) and
  null arms differ per kind (`v == NULL_I64` at `:106` vs
  `v.is_nan()` at `:126`) — so "only the tag skeleton is common"
  is accurate, and the factoring boundary (tags in, values/nulls
  stay inline) cuts exactly along the common-vs-varying line.
- `src/contract.rs:9-12` (`Kind`), `src/xlsx.rs:279-295`
  (per-column `match` into `i64_cell` / `f64_cell`), and
  `DESIGN.md` Templates (§48-62, "Monomorphization is the only
  allowed branch") read as cited. The infinity refusal sits in
  `sheet_xml` ahead of `f64_cell` (`src/xlsx.rs:286-292`),
  untouched per the requirement.
- The helper contract — tag parts only (`out`, `col_letters`,
  `row`), never a value, `Kind`, or `ColumnData`; no trait
  object, no `dyn`, no per-cell match on kind — is sufficient to
  preserve the property: nothing the helper receives can
  discriminate on type, so no dispatch on type can enter through
  it. Keeping both value spellings and both null arms inline at
  the call sites keeps the existing monomorphic shape.

Advisory (not a rejection): the parenthetical "or the equivalent
minimal shape the reviewer accepts" leaves room for a single
helper with a mode/bool parameter, which would trade the three
call-shaped entry points for a per-cell branch — not a branch
*on type* (so literally allowed by R2's wording), but still a
new per-cell branch the mechanism does not need. The applicator
should prefer distinct tiny entry points (open / blank-close /
value-wrap) with no mode flag, and the applicator-phase reviewer
enforces it under R2's "lists every branch" test. If the
applicator wants a mode flag, that is a `DECISION.md`-recorded
deviation with a measurement, not a silent choice.

### R3 — byte-identity, suite green unmodified

The gate list matches the binding protocol: round-trip plus the
stored-zip-verbatim-embed check (`TESTING.md:49-51`, the floor
oracle), `golden_small_archive_is_byte_identical`, the
`static_parts_are_pinned` / `sheet_cap_holds` pins
(`TESTING.md:184-202`), the `src/emit.rs` unit trio, and the
`tests/sealed.rs` locks. "No new in-tree tests … identical bytes
by construction, for reviewer confirmation" is the correct
division: the skeleton respelling is behavior-identical by
construction, and the unmodified suite is the proof. This
review's gate re-run (§Gate) greens the full suite on the
unmodified tree, so the baseline the applicator must preserve is
confirmed fresh, not inherited.

### R4 — bench proof the factoring is free

Protocol verified end to end:

- Bench inventory (`benches/fill_write.rs:84-113`, read):
  `floor` = `header_only`, `wide_null`, `tall_null_{1000,20000,
  200000}` (5) and `cells` = `tall_ints_` / `tall_floats_` at the
  same three scales (6) — eleven total, exactly the SPEC's list.
- Recorded baselines recompute from `TESTING.md`: floats Post-007
  0.187ms / 1.257ms / 12.62ms (`:383-385`), ints Post-005
  0.164ms / 0.870ms / 10.18ms (`:328-332`). Deltas cited there
  (−6.3% / −29.8% / −26.3% floats; −66.7% / −88.5% / −87.7%
  ints) match the 007 review's recomputation.
- Harness rules cite the real hazards: bench profile
  (`Cargo.toml:9-13`, `overflow-checks = true`,
  `codegen-units = 1`), scored criterion harness with the
  `--bench` single-filter flag and the estimates-mtime check
  (`TESTING.md:60-65`), dual baselines (criterion's stored vs
  recorded means — the 007 review §§R4 showed these genuinely
  differ, so the both-baselines rule resolves a real ambiguity),
  and STYLE rule 9 (machine/profile/commit or gossip).
- `#[inline]`-only with `inline(always)` refused encodes the
  cycle 005 lesson (forced inlining regressed 28% and was
  reverted, `TESTING.md:230-236`) — the SPEC carries the ruling
  in rather than re-learning it. "No regression at any of the
  eleven benches or the cycle does not ship" is a falsifiable
  ship gate.

### R5 — comment minimalism

Consistent with STYLE's checked-claims axis (rule 3: an
annotation is a sentence the compiler checks, a comment one it
cannot) and the 007 R1 keep/remove precedent: each surviving
line must state something no compiler-checked artifact states,
the helper gets at most its one-line contract, and the reviewer
lists every touched comment line with keep/remove plus covering
check. The forced-line exemptions (`f64_cell` `is_err` arm
reason at `src/emit.rs:137-138`, `push_magnitude` UTF-8 note at
`:95`) name the exact lines, so the applicator cannot misread
scope. `cargo fmt --check` clean is in the gate (§Gate).

## Gate evidence

`cargo fmt --check`: clean (exit 0).
`CARGO_HOME=/tmp/cargo-home sh check.sh`: "all gates green" —
lib unit tests, `tests/roundtrip.rs`
(`examples_round_trip_through_disk` incl. the verbatim-embed
assertion), `tests/sealed.rs`, zero failures;
`clippy --locked --all-targets -- -D warnings` clean;
`cargo doc --locked --no-deps` clean. Suite unmodified (hash
proof below), so this run is the R3 baseline the applicator must
reproduce.

## Zero-edit hash proof

`git hash-object` after all probes and runs, identical to the
pre-review baseline taken before the gate run:

- `src/emit.rs` `7b5f963f3fd82caa7dbb73803910457673a6ca14`
- `src/xlsx.rs` `bde3abd077e1a03e3956a587cd6f8c655b78a4d5`
- `src/zip.rs` `6914af825864e14dd9daf6973a282e839156e219`
- `src/contract.rs`
  `dd100e62b95fc2ebd4dfd65c2f1f4a52e8db04e9`
- `TESTING.md` `08021c4e55deb9b9ae2d593e01ba3e1d6cbf1c04`
- `STYLE.md` `df5d3676fa3ff5f65b9ba4e5f94489af93b07d00`
- `DESIGN.md` `5b3d562521bbac28c353b18a2f14ca3cf064519c`
- `Cargo.toml` `08e5540084d9fe529e2ff7062c5d794ecad98c67`
- `check.sh` `ce400d11eb8dfd2c08f6b84e14c345d89aec1d0b`
- `benches/fill_write.rs`
  `b6e9c66a8e8798e5c230d2fd3606e594c4655d23`

`git status --porcelain`: only `?? specs/009-skeleton-factor/`.
`git diff HEAD --stat`: empty. No writes outside the new spec
directory except git-ignored `target/` output.

## Findings (none blocking)

1. The `src/emit.rs` line citations in the SPEC Background
   (`:105-119`, `:125-142`) match the file exactly — no drift.
2. The eleven-bench count (six `cells` + five `floor`) matches
   `benches/fill_write.rs:84-113` exactly; the SPEC neither
   omits nor invents a bench.
3. The "no `STYLE.md` register change unless the reviewer rules
   one" clause: ruled — none needed. R5 applies existing STYLE
   rules 1–3 (no caller-less code, checked claims over comments);
   no new convention is created, so no register entry is owed.
