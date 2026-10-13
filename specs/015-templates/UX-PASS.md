# Cycle 015 — UX pass (recorded after human review began; runs before 015 commits)

Sugar only. The human accepted the template contract's semantics
and rejected its declaration surface as unfinished. This pass
adds declarative sugar over the shipped types and changes no
semantics, no emitted bytes, and no contract behaviour.

## Guardrails

- `macro_rules` only. No proc-macro dependency, no build step.
- Every macro expands to exactly the plain `StaticSheet` /
  `DynamicSheet` values a user could write by hand today. The
  expansion is the API; anyone may drop the sugar without
  changing a byte of output or a line of downstream code.
- Kind is stated once, in the column token, and derived from the
  data spelling — the `Column.kind` / `ColumnData` double
  declaration (and with it the `KindMismatch` class for macro
  users) becomes unrepresentable through the sugar. The plain
  constructors keep the boundary refusal for hand-written code.
- Counts are inferred from the literal; no `StaticSheet<2, 3>`
  annotation is required of the user.
- The unknown-dimension spellings stay explicit in the sugar
  (`unknown_rows` / `unknown_columns` visible at the call site):
  honesty about shape knowledge is not hidden.

## Target spellings (design intent)

```rust
let sheet = static_sheet!("fixed",
    i64 "t"  => [100, 200, 300],
    f64 "px" => [10.5, 11.0, 12.25],
);

let sheet = dynamic_sheet!("data",
    i64 "t"  => &t,
    f64 "px" => &px,
);
```

## Forward compatibility (grammar reserved now, implemented only with its kind)

The grammar must not need redesign when the permitted surface
arrives. Column spec shape, reserved from the start:

```
<kind> ["[" <style> {"," <style>} "]"] <name> "=>" <data>
```

- Kinds: `i64`, `f64` today; `str` and `bool` reserved for the
  string/bool cycles (DESIGN Scope: primitives only). An
  unimplemented kind token must fail at compile time with the
  kind named, never expand to a wrong-typed column.
- Styles: cells may be bold, coloured, or both, from the small
  fixed palette (DESIGN Scope). Style markers in brackets after
  the kind — e.g. `f64 [bold] "px"`, `i64 [red, bold] "t"` —
  are reserved syntax now; the style cycles implement the
  markers, this pass does not invent their semantics.
- The header row's existing bold spelling (style 1) is a style
  fact the grammar must be able to state later without a new
  form.

Reserving grammar is not implementing features: no string, bool,
or styled emission ships in this pass (STYLE rule 1).

## Scope when run

Macro definitions (one home, recorded), examples and in-tree
callers migrated to the sugar where it reads better, tests that
the expansions equal the hand-written values byte for byte, and
compile-fail expectations for an unimplemented kind token if the
test harness can state them without new dependencies. Plain
constructors remain fully supported and tested.

Sequencing: after the human finishes the 015 review and before
the emitter cycle (016). Everything left uncommitted for the
human, as with the parent cycle.
