# Syntax

## Literals

| Kind     | Syntax                        | Notes |
|----------|-------------------------------|-------|
| bool     | `true` `false`                | |
| int      | `42` `-7`                     | Optional leading `-`, then ASCII digits |
| float    | `3.14` `-0.5`                 | Optional leading `-`; must contain `.` |
| char     | `'a'` `'👮'`                  | Single Unicode grapheme cluster, enclosed in `'` |
| string   | `"hello"`                     | Unicode text; backing store is bytes (C-compatible) |
| label    | `foo` `my_thing`              | Bare identifier; resolved lazily (see [Types](types.md#labels)) |

## Templates

| Kind     | Syntax                        | Notes |
| -------- | ----------------------------- | ----- |
| list     | `[1, "two", '3']`             | Enclosed in `[]`; element types need not match; bare labels inside are stored as `Label` values — use `$name` to embed the resolved scope value instead |
| set      | `{1, 2, 3}`                   | Enclosed in `{}`; no `:` at the top parsing level; same label/`$name` distinction as list |
| map      | `{x: 1, y: 2}`                | Enclosed in `{}`; `:` at the top parsing level; keys are always label *values*; value positions share the same label/`$name` distinction |
| substack | `(16 16 mul! sqrt!)`            | Enclosed in `()`; a callable value, invoked with `!` |
| script   | `<"hi" print! "there" print!>`| Enclosed in `<>`; like substack but sequential execution guaranteed |

In all templates any value can (in addition to literals) be:
- a template
- a `$` prefixed positive number, for example `$1`
- a `$` prefixed label, for example `$foo`
- an isolated snippet of code followed by an invoke (may use any of the above)

### Nested templates

If the parent template isn't an executable the nested template is rendered when
the parent is using the same scope as the parent.

If the parent template is an executable (substack or script), the nested
template isn't rendered when the parent is rendered. It is instead rendered
when the parent is executed, and thus uses the scope of that parent's execution.

### `$<number>`, stack value

Takes the value `<number>` steps before the template and replaces itself with
it.

**BEWARE**:
Taking `$<number>` means that all values between the template and the number are
consumed! If you don't wish to drop a value between the template and a value
moved into it you need to [reorder the stack](tricks/REORDERING.md) first.

### `$<label>`, scope value

Copies the value bound to `<label>` in the enclosing scope and replaces itself
with it. Unlike `$<number>`, this does not consume any stack entries; it is a
pure copy from scope.

If the label is not found in scope, rendering fails with an error.

### Nested invokes

Same as nested templates, these are only executed for templates that aren't
themselves executable.

An isolated snippet of code followed by `!` inside a template is evaluated at
render time and its result is spliced into the template in place of the snippet.
This allows arbitrary computation during rendering, such as calling built-ins or
previously defined substacks, so long as they return a singular value.

```
[1, 2, 1 2 add!]   # renders to [1, 2, 3], addition is performed at render time
```

The snippet may reference `$<number>` and `$<label>` values from the parent
template, and may itself contain nested templates. This works since rendering
occurs before invoking snippets.

## Comments

`#` begins a comment; the rest of the line is ignored.

```
42 # this is a comment
```
