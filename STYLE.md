# STYLE

2026-10-10. A living record, refined as we go — never finished, never
legislation. Distilled from the Delos `portable_guide.md` and
`design/STYLE.md`, adapted to this project: where this document and the
code disagree, one of them is wrong and the next refinement decides
which. Companion docs: `DESIGN.md` holds decisions, `TESTING.md` holds
protocol and numbers; this file holds philosophy.

Each rule carries the mistake it encodes, decompressed. A rule whose
mistake cannot be named is idiom, and idiom is not an argument.

## 1. Minimal scope, maximal quality

Nothing enters without a caller: every type, verb, config, dependency,
and optimization needs a real consumer, today. A test as spec counts.
`Useful to have` does not. Speculative code is deleted on sight;
re-adding costs one commit, carrying costs permanent debt. The rule
bounds scope, not quality: a faster formulation of an already-needed
job is not speculation — it enters with a measurement, never a belief.
No config without a caller: fixed constants over knobs, and the first
mutable setting is a design event. Size tripwires: no file over 700
lines, no function over 100 lines; break only by written annotation on
the item arguing performance or minimalism benefit.

## 2. Idiom is not an argument

Idiom is compressed experience with context stripped. Decompress it:
name the mistake it encodes, check that mistake is reachable here.
Applies symmetrically to language culture and domain lore — including
spreadsheet lore. Excel practice is evidence, never authority.

## 3. Explicit beats implicit

Two operational tests. Cover test: cover everything but the line —
every name, type, and conversion on it is spelled there or resolvable
by fixed rule from that line alone. Diff test: no line's meaning
changes without that line in the diff. Explicit is not verbose: the
axis is dereference count, not token count. Inference may elide
repetition, never information. Prefer checked claims: an annotation is
a sentence the compiler checks, a comment one it cannot.

Portable Rust spellings, all in force:

- Annotate a binding unless the right-hand side names the type. Never
  annotate what the right-hand side already names.
- No glob imports, tests included. Types by name, free functions
  module-qualified so origin is on the line.
- Conversions spell the target on the line (`T::from(x)`), never bare
  inference (`.into()`). One narrow exception: error propagation where
  the conversion is unique per type pair and defined beside the target
  type.
- Wildcard arms refuse, never handle, on domains this project
  controls — spell every variant so additions break compilation. The
  one standing exception is specified passthrough over an unbounded
  domain (XML escaping copies what it does not encode).

## 4. Machine level, not code level

Simple code is no evidence of simple machine work — a terse conversion
can hide locale tables, allocation, and branches (`stoi` is the
posterboy). Judge every hot path by what the machine does: passes over
bytes, allocations per run, mispredicts per cell. The ideal is each
byte touched exactly once, lined up and done. Branch count is
folklore, not a metric: well-structured predictable branches
comfortably beat branchless contortions on modern hardware, so counted
mispredicts and the bench decide, never branch tally. A claim about
speed that cannot point at fewer passes, fewer allocations, or fewer
mispredicts is aesthetics, and aesthetics loses to the bench.

## 5. Below the threshold, bytes rule

Above the threshold domain types rule; below it (disk, memory, cache)
types cross as labels, never movers. Absence lives inside value bits
(`i64::MIN`, NaN), not sidecar structures, keeping payloads contiguous
and mappable. Buffers are sized once from known lengths; never
zero-then-fill hot buffers. Hot loops are written to vectorize, then
checked for it. Never inherit line, page, or cache constants — measure
on the actual target.

## 6. One representation

Heap and disk share bytes wherever the format allows. Stored layout is
explicit-offset, frozen by round-trip and verbatim-embed tests at first
write. Never store language layout in bytes; declare endianness and
refuse loudly what is unsupported. Write-once publishing: existing
destinations refused, bytes land beside the target and rename into
place. Durability is caller-owned, never fused into publish.

## 7. The oracle stays

One scalar spelling of truth with total semantics: nulls blank,
overflow stops, domain refusals in the same words everywhere. The naive
spelling is kept permanently as oracle — the stored-zip floor and its
verbatim checks — and every faster path is tested bitwise against it.
A tier that cannot show byte-identical output does not ship.

## 8. Wrong is worse than stopped

Overflow panics in every profile, release included. Refuse loudly with
words and location; never trim, coerce, skip, or auto-repair.
Taxonomy: panic is programmer error, `Err` with words is a data
condition, resource death is a machine-exceeding request (no invented
guard). Library code never unwraps on data; `unwrap` and `expect`
live only in tests, benches, and examples, where a panic is the
assertion.

## 9. Measurement is definition

Ship the harness: the floor bench measures everything except
formatting, so formatting work always has something to beat. Best of N
within a run; only within-run ratios are load-bearing, and any
cross-run ratio states its spread or it is not a measurement. Baseline
before a change, compare after, optimize only what the numbers show
matters. A result without its machine, profile, and commit is gossip.
Performance model lives apart from contract (`TESTING.md`, not
`DESIGN.md`).

## 10. Reproducible gate

Release profile with `overflow-checks = true` and `codegen-units = 1`,
each commented with why. Pinned toolchain: codegen is measured
surface. One gate command, exact: format check, tests, clippy as
errors, docs. Dependencies minimal and individually justified; a new
one is a design event under rule 1.

## Rule strengths

Every rule declares one of three strengths, and the strength is part
of the rule: the language checks it (unrepresentability beats
visibility), convention recovers it (costs review attention forever,
listed so the cost is visible), or the style refuses the feature. Rust
runs mostly at strength 1, which is much of why it is the house
language here too.

## Rust chapter (descriptive, 2026-10-10)

What is in Thoth is how it will be; this chapter describes the code,
and the reviewer holds new code to it:

- Imports item-by-item; no bare `.into()`; the single `From` impl
  (build error into write error) sits beside its target under the
  conversions exception.
- No `unwrap`/`expect` in `src/`; errors carry words and positions.
- Capacity is computed from witnessed lengths before any hot loop.

## Register

| # | Date | Question | Status |
|---|------|----------|--------|
| 1 | 2026-10-10 | Glob imports, tests included | **Closed in 002: removed.** Both `use super::*` (`src/emit.rs`, `src/zip.rs` test modules) replaced with itemized imports; glob imports refused everywhere, tests included. |
| 2 | 2026-10-10 | `WriteError::source` handling wildcard | **Ruled: refused.** Remaining variants spelled explicitly (`Exists`, `TempExists`), so the next added variant breaks compilation. |
| 3 | 2026-10-10 | Pre-sized buffers with zero measured gain | **Ruled: kept as hygiene.** One allocation from witnessed lengths; costs nothing, removes a class. Gains must still measure or they are not claimed. |
| 4 | 2026-10-10 | Branch count as a speed metric | **Ruled: refused.** Predictable branches beat branchless code on modern CPUs; mispredicts are measured, branches are not counted. |
| 5 | 2026-10-10 | Hot loops justified at assembler level | **Ruled: required.** The suspicion names the loop, the `.s` names the verdict; const-evaluated code is never "optimized" (004: table builders run zero times, tables are rodata). A vectorized loop is still suspect until the numbers clear it (004: already-vectorized bitwise CRC 69x slower than `crc32x`). |
| 6 | 2026-10-11 | Relative size as cost justification | **Ruled: refused.** A cost is an absolute figure with machine, profile, and commit, never a ratio against an unrelated total (005: "kilobytes against megabytes" struck from the spec; the static copies stay on 117.3ns absolute plus the churn a borrow needs, not on their share of a build). Proof precedes the fix: the lock, refusal test, or measurement is written first and names the change it demands (005: the capacity bound failed on extreme floats before any sizing changed). |
