# Cycle 002 — Applicator DECISION

Applicator: child agent-3. Date: 2026-10-10.
Scope: `specs/002-style-conformance/SPEC.md`, review
`specs/002-style-conformance/REVIEW.md` (agent-2, overall PASS
conditional on an applicator-side diff-scope check).

## Disagreement recorded before acting

The review's finding 1 (observation, non-blocking) notes
`src/zip.rs:128` uses the unbraced single-item form
`use super::crc32;` rather than the braced `use super::{crc32};`
spelling named in SPEC R2, and suggests normalizing to braces only
if a future touch is already editing that line. I agree with the
reviewer's non-blocking reading and go one step further: I decline
to normalize now. Reasoning: the import is semantically itemized —
the glob is gone, exactly the one name the test body uses is named,
and the rule ("glob imports refused everywhere, tests included")
is satisfied. A braces-only edit would add diff churn with zero
rule benefit and cut against the cycle's minimal-diff constraint.
No source edit is made for this finding.

## Applied (no source changes required)

The tree already satisfies all seven SPEC requirements, as the
review documents with line-level evidence, which I re-inspected and
confirm: `Cargo.toml:15-16` holds only `criterion = "0.5"`
(`Cargo.lock` calamine count 0); `src/emit.rs:88` itemized braced
import of exactly the four used names; `src/zip.rs:128` itemized
single-name import; `grep -rn 'super::\*'` over
`src tests benches examples` returns nothing (exit 1);
`src/xlsx.rs:113-122` `source()` spells
`WriteError::Exists(_) | WriteError::TempExists(_) => None` with no
`_` arm; the seven `: usize` annotations are present and
RHS-naming bindings stay bare; `check.sh:4-7` pins `--locked` on
test/clippy/doc but not fmt; `src/zip.rs:117` carries the
little-endian comment with the CRC-32 IEEE line kept; `STYLE.md`
register rows 1-2 are closed/ruled with rows 3-4 intact. The two
remaining `calamine` mentions (`SPEC.md` task text, `TESTING.md:44,88`
reader oracle) are pre-existing out-of-scope text, not scoped files.

## Calamine decision (SPEC acceptance record)

`calamine` removed per review finding 1 (SPEC R1): deleted from
`[dev-dependencies]` and regenerated out of `Cargo.lock`. It is to
be re-added by the reader-wiring cycle at one commit of cost.

## What changed

- Created this file: `specs/002-style-conformance/DECISION.md`.
- No other files changed: no logic, message, capacity-math, bench,
  expectation, or formatting edits.

## What was rejected and why

- Brace normalization of `src/zip.rs:128` (`use super::crc32;` →
  `use super::{crc32};`): rejected as churn; semantically itemized
  already, rule satisfied, review non-blocking.
- `git diff --stat` scope confirmation: unresolvable here — the
  workspace is not a git repository (`fatal: not a git repository`),
  same as at review time. Substituted by content inspection: the only
  new file under `specs/002-style-conformance/` besides `SPEC.md`
  and `REVIEW.md` is this file, and line-level inspection of every
  scoped file shows only the SPEC-mandated renames, annotations,
  flags, and one comment. A host with history should still run
  `git diff --stat` to close the review's conditional.

## Verification

- `export CARGO_HOME=/tmp/cargo-home` for all cargo commands.
- `cargo fmt` then `sh check.sh`: all gates green
  (`cargo fmt --check`, `cargo test --locked`, `cargo clippy --locked`,
  `cargo doc --locked`; output ends `all gates green`).
