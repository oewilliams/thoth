# Cycle 010 — DECISION (applicator record)

Applicator read `SPEC.md` and `REVIEW.md` in full, then edited only
the scoped files. No library, test, manifest, or style file touched.

## Reviewer verdicts and dispositions

- Cited-body verification (refs, baselines, suite names, protocol
  details): reviewer-verified accurate; applicator re-confirmed the
  two timed sections while editing. No discrepancy. Disposition:
  accepted, applied as specified.
- R1 — APPROVE. Ruling (1): `writer_columns()` is setup, not fill,
  so both harnesses time the `write_xlsx_file` call alone.
  Disposition: applied. `benches/fill_write.rs` `write_iter` now uses
  `iter_with_setup`: setup builds the path, pre-cleans debris, and
  constructs columns; the timed closure holds exactly
  `thoth::xlsx::write_xlsx_file(&path, &cols).unwrap();`. Ruling (2):
  criterion 0.5 has no post-iteration teardown hook. Disposition:
  pre-clean in setup plus one trailing `remove_file` after the run,
  outside the clock; no deletion inside the timed closure.
  `examples/bench.rs`: `remove_file` and `writer_columns()` moved
  above `Instant::now()`; the clock spans the `write_xlsx_file` call
  only. Groups, recipes, scales, criterion version untouched.
- R2 — APPROVE, with tightenings (operational same-spell reading;
  3% strictness accepted as churn-risk, never false acceptance).
  Disposition: standing rule text written into `TESTING.md`
  (benchmarks section) verbatim in substance, including both
  tightenings; enforced during the re-takes — consecutiveness
  required, and where it was not achieved the gap is named in the
  Post-010 entry rather than the rule loosened.
- R3 — APPROVE. Disposition: uptime, 1-minute load, and free pages
  recorded beside every 200k figure in the Post-010 entry
  (commands re-runnable: `uptime`, `vm_stat`).
- R4 — APPROVE, sufficient. Disposition: full gate green
  (`cargo fmt --check`; `sh check.sh` with
  `CARGO_HOME=/tmp/cargo-home`: tests, `clippy -D warnings`, docs),
  named suites green unmodified, byte-size probe matches 003
  figures, `git diff --stat tests/ src/` empty (verified output).
- R5 — APPROVE. Disposition: re-takes done under the R1 harness via
  the scored harness (direct binary, single filter, `--bench`;
  estimates mtimes confirmed moved). Floats: full protocol pass,
  span 0.134ms vs the 2.39ms Post-009 bar. Ints: agreeing reads
  recorded with a named consecutiveness gap. Floor: no qualifying
  read, no number claimed — box evidence recorded, control queued
  for next cycle. Guardrail honored: moved clocks attributed to
  harness accounting (unlink probe: median 164µs, p90 503µs, mean
  3.4ms per 10MB unlink, two per old iteration) plus machine state;
  no library speedup claimed.
- Scope / boundaries — APPROVE. Disposition: honored. Modified:
  `benches/fill_write.rs`, `examples/bench.rs` (clock lines only),
  `TESTING.md` (rule text + Post-010 entry). New: this file.
  Untouched: `src/`, `tests/`, `Cargo.toml`, `Cargo.lock`,
  `STYLE.md`, `DESIGN.md`. No new tests, benches, or dependencies;
  probes in `/tmp` (`/tmp/byteprobe`, inline unlink probe).
- Overall — APPROVE, no rework. Disposition: no rework needed; the
  two inherited rulings are applied above.

## Deliberate no-changes

- The `examples/bench.rs` print header still reads `fill+write`
  although the clock is now write-only. Cosmetic label, outside the
  "clock lines only" scope; the timed lines (what the review
  checks) are correct and named in the Post-010 entry.
- Doc comments describing "fill plus write" left as-is: fill names
  the `write_xlsx_file` call's internal work, which is still
  exactly what the clock covers.

## Verification evidence

- `cargo fmt --check`: clean. `cargo bench --bench fill_write
  --no-run`: compiles (criterion 0.5.1).
- Gate: `all gates green` (fmt, `cargo test --locked`, clippy,
  doc). Suite: 18 unit + round-trip + 4 sealed green.
- Bytes: 48001 / 50002, 990005 / 1030006, 10470009 / 10870010 —
  all match 003 figures.
- Everything left uncommitted: no commit, push, amend, or tag.
