# Cycle 015 — Review (template contract)

Reviewer, independent of the producer. Tree snapshot before review:
`git status` showed the 11 modified + 2 untracked producer files and
nothing else; `specs/015-templates/` held only `SPEC.md`. This review
adds only this file (verified at the end). Machine: Darwin 27.0.0
arm64 (`RELEASE_ARM64_T8132`), rustc 1.93.1, HEAD `c03f374` plus the
uncommitted 015 work — same statement as the Post-015 entry.

## Overall verdict

**Conditional pass.** The contract, both impls, monomorphisation,
routing, byte-identity, and the no-regression record all hold on my
own runs. Three narrow required changes (RC1–RC3) and one comment
restoration (RC4) stand before the applicator ships; two records are
applicator-stage by design (DECISION.md, DESIGN.md). Per-requirement
verdicts below; every claim names the inspected file and line or the
probe that produced it.

## R1 — One contract, declared once: PASS, with RC1

One declaration: `SheetTemplate` (`src/contract.rs:168-180`) takes
`sheet_name`, `shape`, and `columns_view` (name + data per column,
`ColumnView` at `src/contract.rs:158-161`); shape knowledge is
`ShapeKnowledge { rows: Option, columns: Option }` with the unknown
dimension named (`src/contract.rs:94-145`). Consumers do not
re-derive the reservation case: `sheet_hint` reads the declared row
count only (`src/contract.rs:383-384`), the builder reserves from it
before any emission (`src/xlsx.rs:357-369`), and `sheet_cap` is the
worst-case proof, never the reservation (`src/contract.rs:356-375`).
Rectangularity is witnessed once in `validate_template`
(`src/contract.rs:331-354`), as DESIGN Ingress requires; the ceiling
consumes that witnessed count (`src/xlsx.rs:359`), which coincides
with the declared count by construction for both in-tree impls
(`src/contract.rs:195-201`, `src/contract.rs:302-304`).

Finding (RC1): kind is declared twice and only one spelling is read.
`Column { name, kind }` (`src/contract.rs:36-39`) coexists with the
`ColumnData::I64/F64` variant, and every driver keys off the data
variant (`src/contract.rs:367`, `:389`, `:425`, `:481`); `ColumnView`
drops the declared kind entirely. My probe built a column declared
`Kind::F64` over `ColumnData::I64` data: `sheet_xml_template`
succeeded and emitted integer cells — no refusal anywhere. The
pre-015 tree had the same duplication, but R1 restates kind-at-
declaration as the contract's first clause, so the contract must
enforce it. **RC1: `validate_template` refuses a column whose
declared `Column.kind` disagrees with its `ColumnData` variant (or
the applicator removes the duplicate spelling and records why in
DECISION.md).**

## R2 — Two impls, one emitter set: PASS

Dynamic is the pre-015 behaviour behind the contract
(`DynamicSheet`, `src/contract.rs:186-249`; `Sheet` is its alias,
`src/xlsx.rs:33`). Static is const-shaped (`StaticSheet<C, R>` over
`[StaticColumn<R>; C]`, `src/contract.rs:282-319`). Both drive the
identical generic drivers `emit_sheet` / `emit_sheet_arena`
(`src/contract.rs:403`, `:450`), which call the same `src/emit.rs`
cell spellings. No `dyn SheetTemplate` exists (the only `dyn` in the
tree is the pre-existing error-source spelling,
`src/xlsx.rs:196`); no new `unsafe` (only the untouched CRC
intrinsics, `src/zip.rs:101-104`). The hot loop
(`src/contract.rs:479-492`) matches only the column's data variant —
the per-column kind decision, as before — with no branch on impl or
shape; each sheet's views are gathered once before the loop
(`src/contract.rs:463`), which the Post-015 entry credits for
removing the first draft's 10–15% null regression.

Assembler/symbol evidence (reviewer's own): a `/tmp` probe with
`#[inline(never)]` wrappers around `build_workbook` for each impl
produces two distinct instantiations (`build_dyn`, `build_stat` in
the probe binary), drivers inlined into each, and no
`SheetTemplate` vtable (the only vtable shims are core `FnOnce`).
The static example's release assembler inlines the whole emission;
its indirect calls are core drop glue, not trait dispatch. Static
caller ships: `examples/static_sheet.rs` (27 lines; my run printed
`3569 bytes`) plus the disk round-trip at `tests/template.rs:122-151`
and static==dynamic bytes at `tests/template.rs:91-120`.

## R3 — Hint or named unknown, from the declaration: PARTIAL (RC2, RC3)

Holds for rows. Known shape hints at expected widths (10 `I64` /
12 `F64`, `src/contract.rs:389-392`) against the cap's widest
(20 / 327, `:367-370`); `tests/template.rs:77-89` pins hint < cap and
`sheet_hint(unknown) == None`. Unknown rows name the dimension
(`src/contract.rs:207-213`), the builder falls back to `ARENA_BLOCK`
(`src/xlsx.rs:360`) with the case decided before emission
(`src/xlsx.rs:368-377`). Retained capacity is recorded, null 1.93x
reported with the successor decision named, not taken
(`TESTING.md:1024-1038`). Growth is priced honestly at +6–12%
(`TESTING.md:1066-1073`) against SPEC's "014 priced it at parity"
background — recorded, not hidden; that background claim does not
survive contact and the entry says so in numbers.

Two gaps. (a) Unknown *columns* is declared
(`DynamicSheet::unknown_columns`, `src/contract.rs:217-224`) but
never consumed as unknown: my probe shows
`sheet_hint(unknown_columns) == Some(20200401)`, identical to the
known spelling — the reservation re-derives the witnessed columns
from the data, contradicting R1's consume-without-re-deriving
clause, and no test exercises the spelling. **RC2: `sheet_hint`
returns `None` when either dimension is unknown (builder growth,
same as rows), with a byte-identity test for the columns spelling —
or the applicator narrows the contract to rows-only and records the
narrowing in DECISION.md.**
(b) The in-tree unknown-rows tests compare bytes only at 2–3 rows
(`tests/template.rs:40-75`), far under the initial reservation, so
no in-tree test forces the arena to grow on the unknown path. The
behaviour itself is sound — my 200k mixed probe: known vs unknown
byte-identical, capacities 20203485 vs 23093248, hint
`Some(20200401)` vs `None` — but the lock lives in a `/tmp` probe,
not the gate. **RC3: add an in-tree unknown-rows build large enough
to exceed the initial reservation (sheet well past `ARENA_BLOCK`)
asserting byte-identity with the known spelling.**

## R4 — Emission driving behind the contract: PASS

`src/emit.rs` is unchanged: `git diff` empty and sha256
`eae5d022…` matches HEAD exactly. `sheet_xml` remains the oracle
spelling, implemented through the contract
(`src/xlsx.rs:267-277` → `emit_sheet`). Row loop, letters
(`src/contract.rs:323-325`), row digits, and per-row CRC folds
(`src/contract.rs:469-499`) all sit behind the contract surface.
Error semantics verified by probe, not just reading: a ragged second
sheet beside an infinite first sheet refuses at the boundary
(`sheet "Second": column "b": ragged length 1, want 2`) before any
emission, and an infinite at data row 2 surfaces wrapped per sheet
in row order (`sheet "S": column "px" row 3: infinite is not an
excel value`); all-sheet validation precedes emission at
`src/xlsx.rs:325-334`.

Non-blocking style finding (RC4): the `src/xlsx.rs` rewrite deletes
substantial binding rationale (the `archive_fits` cast-cover
enumeration, the `sheet_cap`/`sheet_hint` derivations) that
Post-005/014 recorded as load-bearing documentation. Scoped files
permit the edit, but STYLE holds docs and code to the same bar.
**RC4: restore the trimmed rationale (the cap derivation survives
in part at `src/contract.rs:356-360`) or record the deviation in
DECISION.md.**

## R5 — One route, and byte-identity: PASS (records pending)

Every in-tree caller routes through the contract. My greps for the
old entry spellings: no `Sheet { … }` struct literal survives
anywhere (constructors only, `src/contract.rs:197-224`); the
remaining `sheet_xml` / `build_xlsx` / `write_xlsx_file` hits are
exactly the compat definitions (`src/xlsx.rs:267`, `:385`, `:419`),
each a one-line delegation through `DynamicSheet::known`, plus
their byte-pinning test (`tests/template.rs:153-168`) and
`wrapper_matches_direct_workbook` (`src/xlsx/tests.rs:107-121`).
`validate_columns` is gone. `write_xlsx_file` is now uncalled
in-tree (sealed tests migrated to `write_workbook_file`); it stands
as a kept spelling by delegation, acceptable under R5, and worth
one line in DECISION.md. Note the phase bench now times
`sheet_xml_template` where it timed `sheet_xml`
(`benches/fill_write.rs`) — the same bytes through the contract
spelling, consistent with TESTING's protocol.

Signature change: `build_workbook` / `write_workbook_file` are now
generic over `SheetTemplate` and `Sheet` is an alias constructed
only via `DynamicSheet::known/unknown_rows/unknown_columns`
(private `shape` field makes struct literals unrepresentable).
R5's record of that change belongs in `DECISION.md`, which does
not exist yet by workflow design; likewise the DESIGN Templates
update (`git diff -- DESIGN.md` empty). **Applicator-stage, not a
producer defect: DECISION.md must record the generic-signature
change with the caller migration, and DESIGN.md must be updated
to what shipped.**

Byte-identity, reviewer's own runs: `sh check.sh` green (format,
33 lib tests + round-trip + multi-sheet + 4 sealed + 6 template,
clippy `-D warnings`, docs). A HEAD probe (`git archive` extraction)
against a post-015 probe, `cmp` at 200,000 rows: ints 10470009,
floats 10870010, null 10470071, mixed 17047851, three-sheet —
identical on every recipe (my three-sheet recipe reads 21338066 on
*both* sides; Post-015's 21338118 is a differently-built header
sheet, and the load-bearing fact is HEAD==post on one recipe).
Returned capacities reproduce Post-015 exactly (ints 12203418,
floats 12603419, null 20203485 = 1.93x, mixed 20203485).

## R6 — Measured no-regression: PASS, with named gaps

Post-015 (`TESTING.md:995-1088`) states machine, profile, and
commit, uses the interleaved A/B the 014 method finding requires,
and claims no speedup: sub-HEAD build reads are recorded as layout
drift with the reasoning on the page (`:1052-1056`), null at
parity. No recipe regresses on any recorded read. The static impl's
benefit is recorded plainly as none measurable at its shape
(1.030–1.033µs static vs 1.043–1.062µs dynamic, filesystem-bound
writes at ~26 MB/s, `:1075-1088`) — the impl stands on ergonomics,
as SPEC demands. My corroboration: capacities and byte lengths
above match the entry to the byte; `cargo bench --bench fill_write
--no-run` and `cargo test --benches` compile, and the static
criterion group exists in-tree.

Gaps, inherited and named: the full criterion phase re-take to the
agreeing-pair protocol was not re-run (the entry rests on the
interleaved probe), and the producer's `cargo bench --no-run`
SIGSEGV is asserted pre-existing on HEAD — I confirm bench sources
build and run here but did not reproduce the compiler crash on
either tree. Reviewer timing is therefore corroboration (bytes,
capacities, gates), not an independent R6 re-take.

## Required changes (precise)

- **RC1** (`src/contract.rs`): kind declared twice, read once —
  refuse `Column.kind` vs `ColumnData` disagreement in
  `validate_template`, or remove the duplicate with the reasoning
  in DECISION.md.
- **RC2** (`src/contract.rs:383`): `sheet_hint` must honour
  unknown columns (`None` → growth) or the contract narrows to
  rows-only on the record; add the byte-identity test either way.
- **RC3** (`tests/template.rs`): an unknown-rows build that forces
  arena growth past the initial reservation, bytes asserted
  against the known spelling.
- **RC4** (`src/xlsx.rs`): restore the trimmed `archive_fits` /
  sizing rationale or record the deviation in DECISION.md.
- Applicator-stage records (not code): DECISION.md records the
  generic `build_workbook`/`write_workbook_file` change, the kept
  `write_xlsx_file` spelling, and the RC outcomes; DESIGN.md
  Templates updated to what shipped. No other changes required.

Tree-identical check: after writing this file, `git status` and the
per-file sha256 set match the pre-review snapshot except the added
`specs/015-templates/REVIEW.md`; all probes lived in `/tmp`.
