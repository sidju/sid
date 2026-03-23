# `typed_args`

**Availability:** comptime + runtime

Sets the `args` type annotation on a substack, script, or `Fn` type value.
Pops a list of types (top) and the target callable (below), and returns the
callable with its `args` dimension set.

## Stack effect

```
... callable [$T1 $T2 …]  →  ... callable
```

The list is on top; the callable is below.

## Using `$label` for type names

Type names like `int`, `bool`, `str` are labels in global scope. A bare label
inside a list literal produces a `Label` value — a distinct type from the
resolved `Type(...)` value. `typed_args` requires `Type(...)` values, so use
`$name` to embed the resolved value at render time:

```
(my_body) [$int $bool] typed_args !   # $int resolves to Type(Int) at render time
(my_body) [int bool]   typed_args !   # int is Label("int") — wrong type for this use
```

## Example

```
(42) [$int] typed_args !
# stack: Substack { body: [42], args: Some([Int]), ret: None }

fn ! [$str $int] typed_args !
# stack: Type(Fn { args: Some([Str, Int]), ret: None })
```

## Errors

- Panics if the list contains non-type values (labels that weren't resolved
  with `$` will cause this).
- Panics if the target is not a `Substack`, `Script`, or `Fn` type.

## See also

- [typed_rets.md](typed_rets.md)
- [untyped_args.md](untyped_args.md)
- [fn.md](fn.md)
