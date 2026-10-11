# REVIEW — Cycle 005 re-review (user, 2026-10-11, chat)

Reviewer: user. This file transcribes the reviewer's verdict
verbatim; the applicator added only the tree-state facts at the
end. No source file was edited by the review.

## Verdict (verbatim)

Ok, I'm gonna review the whole xlsx file. (1) We could probably
move the consts to their own file. (2) Comments are getting a bit
much. They should only be there if forced, and minimal if they
are forced. (3) archive_fits looks like its entierly a boundary
concern wrt the checks. Then we loop over pairs here for the
sake of two sums, which is probably not the most efficient way
around. The addition of consts in a loop can be reduced to a
single multiplication + add. (4) sheet_cap similar, that loop
can definitely be optimised. And if we march a pointer along
letters we dont need the enumerate. (5) sheet_xml... Yeah that
is not a good enough function, reccomend clean up and optimise
in one pass before I re-review.

## Tree state at review (applicator-observed)

`git status --short`:

```text
M STYLE.md
M TESTING.md
M src/xlsx.rs
?? specs/005-foundation-tighten/
?? tests/sealed.rs
```

Gate green at review time (`sh check.sh`, `all gates green`):
lib 17/17, round-trip 1/1, sealed 4/4. Byte output unchanged
except the specified error strings.

## Verdict II (verbatim, same reviewer, same day)

(1) archive_fits - I still dont like that loop, especially the
copy. Also why are we casting to u64 in the loop to then compair
against u32 and u16s? Surely its better to hoist the consts at
compile time. (2) sheet_cap - cap can definitely be done in one
line with an accumulator and not be mut. (3) sheet_cap, still a
bit unsure about that loop and if it can be optimised more, that
match is likely to cause a vectorisation miss. Or are there
other AVX instructions we could use? (4) sheet_xml - those
checks are boundary concerns, by the time we hit this we should
have already guaranteed them. (5) sheet_xml - loop from 255 to
266. Nested loop with conditionals for something that should
probably also sit at the template construction level (to be
designed) or allowed to panic. (6) I keep seing loops over
tuples with dropped sides, this tells me we should use a struct
of arrays not an array of structs. That might help with other
optimisations in loops as well.
