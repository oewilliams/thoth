# Cycle 017 — Review (string and bool kinds)

Reviewer, independent of the producer. Snapshot before review:
per-file sha256 set (79 files, `target/` excluded) recorded in
`/tmp/review-before.txt` (set sha256 `76c1a7ff…`); `git status`
showed the producer's uncommitted work only — `M TESTING.md`,
`M benches/fill_write.rs`, `M examples/bench.rs`,
`M src/contract.rs`, `M src/emit.rs`, `M src/macros.rs`,
`M src/xlsx.rs`, `M src/xlsx/tests.rs`, `M tests/common/mod.rs`,
`M tests/roundtrip.rs`, plus untracked `examples/strings_bools.rs`
and `tests/primitives.rs`. `specs/017-primitives/` held only
`SPEC.md`. This review adds only this file (verified at the end).
Machine: Darwin 27.0.0 arm64 (`RELEASE_ARM64_T8132`), Apple M4,
rustc 1.93.1, bench profile (`overflow-checks = true`,
`codegen-units = 1`), HEAD `1482e88` plus the uncommitted 017
work — same statement as the Post-017 entry (`TESTING.md:1169-1173`).

## Overall verdict

**Pass with two required changes.** The new kinds are spelled,
bounded, sized, and read back correctly: my own probes confirm
the OOXML spellings, the exact 32,767 boundary, the full control
set, kind-mismatch on the new variants, oracle/arena identity,
HEAD-dump identity for existing kinds, macro equality, and a
round-trip reader that provably fails on a flipped byte. Two
defects stand, both narrow: `sheet_cap` is not a true bound when
a column *name* needs escaping (pre-existing at HEAD, but R4
claims a true worst case), and `validate_template` now exceeds
STYLE's 100-line function tripwire without the required
annotation. One timing discrepancy on the null recipe is
unresolved (R6). `DESIGN.md` Scope wording is applicator-owned
per the SPEC's scoped files; the producer correctly left it
untouched.

## R1 — Bool kind: PASS

`Kind::Bool` and `ColumnData::Bool(&[Option<bool>])`
(`src/contract.rs:16-21`, `:35-40`), `StaticColumn::Bool`
(`[Option<bool>; R]`, `src/contract.rs:248-251`) with
`from_static_bool` (`:77-79`), both drivers
(`src/contract.rs:562`, `:607`), and the macro arms
(`src/macros.rs:27-32`, `:69-77`). Spelling is the OOXML boolean
cell: `bool_cell` (`src/emit.rs:161-174`) emits
`<c r="A2" t="b"><v>1</v></c>` / `…<v>0</v></c>`, `None` blank —
confirmed in my probe's sheet dump and by the pinned literal in
the emitter sweep (`src/emit.rs:481-488`). Round-trip: the test
reader's bool branch accepts exactly `"1"`/`"0"` and panics
otherwise (`tests/common/mod.rs:525-531`), `mixed_kinds`
expects `CellValue::Bool` values (`tests/common/mod.rs:320-323`),
and both `mixed_kinds_round_trip_every_kind`
(`tests/primitives.rs:27-36`) and the disk round-trip
(`tests/roundtrip.rs:17-45`) pass on my `sh check.sh` run.

## R2 — String kind: PASS

`Kind::Str`, `ColumnData::Str(&[Option<&str>])`,
`StaticColumn::Str` (`[Option<&str>; R]`) and the `str` macro
arms mirror R1's plumbing. `str_cell` (`src/emit.rs:186-203`)
spells `t="inlineStr"` with the same five-entity table as
headers (`escape_into`, `src/emit.rs:23-34`; byte twin
`escape_into_bytes`, `:258-272`). `xml:space="preserve"` sits on
the `<t>` element exactly when the text leads or trails with
whitespace (`needs_preserve`, `src/emit.rs:178-181`; emitted at
`:195-197`, byte twin `:350-353`). My probe sheet shows
`a &amp; &lt;b&gt; &quot;q&quot; &apos;x&apos;`,
`<t xml:space="preserve"> lead</t>` (also trailing/both/tab),
no `preserve` on plain text, and
`<c r="B3" t="inlineStr"><is><t></t></is></c>` for `Some("")`
beside `<c r="B4"/>` for `None` — empty distinct from blank,
also pinned at `src/emit.rs:505-518`. Control refusal lives in
R3. One reader caveat, not a defect: the byte reader ignores
`xml:space`, so the preserve spelling is pinned by the emitter
sweep, not by round-trip — Excel-side trimming rests on that
pin, which is present and exact.

## R3 — Boundary checks for the new kinds: PASS

`validate_template` runs the string scan beside the float scan,
before any emission (`src/contract.rs:381-440`; both entry
points validate first: `src/xlsx.rs:363-365`,
`:428-434`). The valid set is spelled out where the check lives
(`src/contract.rs:381-390`): tab/LF/CR plus
`0x20..=0xD7FF`, `0xE000..=0xFFFD`, `0x10000..=0x10FFFF`. My
probe swept `U+0000..=U+0020` exhaustively — only tab, LF, CR,
space kept — plus `0x7F`/`0xFFFD`/astral kept and
`0xFFFE`/`0xFFFF` refused, matching the spelled set byte for
byte. The cap counts *characters*: 32,767 accepted (ASCII and
3-byte chars), 32,768 refused with
`column "name" row 3: text is 32768 characters, excel stops at
32767`; control refusals name column and Excel row in the same
message shape (`src/xlsx.rs:188-199`), the earliest row wins
across columns (probe: `bcol` row 3 beats `acol` row 4), and
`build_workbook` refuses wrapped as `InSheet` before emitting.
Kind-mismatch covers the new variants through the existing
first check (`src/contract.rs:334-343`, labels at
`src/xlsx.rs:104-111`): my six-case matrix (str↔bool, str↔i64,
bool↔f64, both directions) all refuse; the producer's test pins
one direction (`tests/primitives.rs:77-94`), the code path is
variant-exhaustive. Style defect on this requirement's
function: see Required change 2.

## R4 — Sizing learns the new kinds: FAIL (narrow, pre-existing)

The arithmetic the cycle added is a true bound. Bool
contributes its fixed spelling (`Kind::Bool => 7`,
`src/contract.rs:474-483`); strings contribute per cell
`6 * chars + tags` from the actual text, preserve tag included
(`src/contract.rs:457-473`) — the 6× ceiling is the widest
entity (`&quot;`/`&apos;`). My worst-case probes, beyond the
extended `sheet_cap_holds` (`src/xlsx/tests.rs:270-300`): a
200-row sheet of padded all-quote/all-apostrophe/mixed-entity
texts reads len 99171 ≤ cap 114074; one 32,767-quote cell reads
len 196901 ≤ cap 197033 (slack 132); a 500-emoji sheet reads
2299 ≤ 3431; bools 9824 ≤ 11735. `sheet_hint` prices witnessed
text bytes plus tags (`src/contract.rs:504-518`); my retained
measurements reproduce the Post-017 entry exactly (R6).

The bound fails on column *names*. `sheet_cap` prices each
header at `letter.len() + name.len() + 64`
(`src/contract.rs:451-453`) while `header_cell` escapes the
name through the same 6× table. Probe (header-only, name =
repeated `"`): 20 quotes reads len 354 > cap 353; 40 quotes
474 > 373; 100 quotes 834 > 433; fifty 20-quote columns read
10999 > 8439. Identical numbers on an `I64` column, and the
formula is byte-identical at HEAD (`src/contract.rs:359` in the
HEAD tree), so this pre-dates 017 — but R4's claim is
unqualified ("stays a true worst case"), and `archive_fits`
rests its cast cover on actual ≤ cap (`src/xlsx.rs:291-295`).
Output bytes are unaffected (the oracle `String` reallocates,
the arena grows); the no-realloc proof and the projection
premise are what break. Required change 1.

## R5 — One spelling per driver pair: PASS

The sweep extension pins oracle (`String`) against arena
(bytes) for bools and for strings with escapable characters,
leading/trailing whitespace, and empty-vs-blank
(`src/emit.rs:474-531`); both new cell functions ship without
`#[inline]`, per the recorded layout episode (R6). My probes:
the arena-built archive embeds the oracle sheet verbatim for a
str/bool sheet, a static str/bool archive equals its dynamic
twin byte for byte, and unknown-rows/unknown-columns builds
equal the known build. Existing kinds unchanged, from my own
runs: fresh `git archive` HEAD extraction behind an identical
`/tmp` probe crate, `build_workbook` dumps at 200,000 rows,
`cmp` HEAD vs post-017 — ints 10470009, floats 10870010, null
10470071, mixed 17047851, three-sheet 21338118, identical on
every recipe and equal to the Post-017 figures
(`TESTING.md:1198-1203`).

## R6 — Measured record: PASS (one discrepancy unresolved)

Post-017 (`TESTING.md:1169-1235`) states the new recipes (tall
strings at 12-char width, tall bools) in the `phase` group
(`benches/fill_write.rs:237-291`), agreeing build pairs
(strings 11.678/11.628ms, 0.4% apart; bools 3.2514/3.2001ms,
1.6% apart, tight CIs, superseded reads named at `:1192-1196`),
machine, profile, and commit, with no speed claim beyond the
reads. Existing recipes are shown two ways (byte-identity,
interleaved parity) and the `#[inline]` episode is recorded,
not hidden (`:1208-1213`). I reproduced the retained figures
exactly: strings 16381122 len / 17003421 cap (1.04×), bools
10781120 / 11403419 (1.06×) (`:1223-1226`), and the archive
lengths above. I did not re-run the criterion pairs; the entry
is protocol-complete on its face.

**Unresolved — null timing.** My interleaved HEAD/post-017
probe (two probe binaries, 5×15 builds, median of medians, six
alternating pairs) reads ints and floats at parity but null
consistently over HEAD: post-017 4.20–4.36ms vs HEAD
3.67–4.02ms (~+10–14%), where the entry records overlapping
parity (`:1204-1208`). Null bytes are `cmp`-identical and the
null path touches no new spelling, and cross-binary layout
drift on this recipe is documented (Post-015), so my probe
cannot attribute the gap to library cost versus layout — but
it does not corroborate parity either. The applicator should
record in `DECISION.md` which reading it accepts and why,
or re-take the interleaved pair in the producer's harness.
No verdict above depends on this item.

## Macro grammar, reader genuineness, gates (cross-cutting)

Macro `str`/`bool` expansions equal the hand-written values on
both sheet macros (my probe, matching
`tests/primitives.rs:153-186`), and styled tokens still fail:
`/tmp` snippets `str [bold]` and `bool [red]`/`[bold]` on both
`dynamic_sheet!` and `static_sheet!` all fail with
`column styles are reserved syntax and are not implemented
yet` (`src/macros.rs:33-35`, `:78-80`), while plain `str`
compiles; the compile-fail doctest moved to the styled
spelling (`src/macros.rs:107-111`) and passes. The round-trip
reader genuinely reads the new kinds: flipping one stored
bool byte `1`→`0` makes `read_sheet_cells` return `Bool(false)`
against `Bool(true)` expected, and flipping one text byte
changes the read-back — the reader fails on a wrong byte, not
just passes on the right one (`tests/common/mod.rs:489-541`).
My `sh check.sh` run is green (fmt check, 34 lib tests, 6
primitives tests, round-trip, multi-sheet, sealed, template,
clippy `-D warnings`, docs); `examples/strings_bools.rs` runs
(3711 bytes), `static_sheet` still 3569 bytes, and the bench
example's read-back asserts pass including `mixed_kinds`.
Scoped files only; `Cargo.toml`/`Cargo.lock` untouched, no new
dependencies, no `unwrap`/`expect`/bare `.into()` added in
`src/`, no new `unsafe`.

## Required changes

1. **`src/contract.rs:451-453` (`sheet_cap`): price header
   names at their escaped bound** (6× per character plus the
   fixed header tags, as the data cells already are), or
   otherwise restore `len <= cap` for escapable names; extend
   `sheet_cap_holds` (`src/xlsx/tests.rs`) with an
   escapable-header sheet. Pre-existing at HEAD; output bytes
   unaffected; R4's worst-case claim and the `archive_fits`
   projection premise are what fail.
2. **`src/contract.rs:329-441` (`validate_template`): 113
   lines exceeds STYLE §1's 100-line function tripwire with no
   written annotation** (HEAD's version was 54 lines). Extract
   the string scan (`:381-435`) into a named helper mirroring
   the float scan, or annotate the item with the required
   argument. No behavior change either way.

Applicator-stage records, not producer defects: `DECISION.md`
must carry the R6 null-timing resolution above, any
disagreement with Required changes 1–2 and its reasoning, and
the `DESIGN.md` Scope wording update the SPEC assigns to the
applicator.

Tree-identical check: after writing this file, `git status`
and the per-file sha256 set match the pre-review snapshot
except the added `specs/017-primitives/REVIEW.md`; all probes
lived in `/tmp` (`/tmp/review017/`).
