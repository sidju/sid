# `typed_rets`

**Availability:** comptime + runtime

Sets the `ret` type annotation on a substack, script, or `Fn` type value.
Pops a list of types (top) and the target callable (below), and returns the
callable with its `ret` dimension set.

## Stack effect

```
... callable [$T1 $T2 …]  →  ... callable
```

## Using `$label` for type names

A bare label inside a list literal produces a `Label` value — a distinct type
from the resolved `Type(...)` value. `typed_rets` requires `Type(...)` values,
so use `$name` to embed the resolved value at render time:

```
(my_body) [$bool] typed_rets !   # $bool resolves to Type(Bool) at render time
(my_body) [bool]  typed_rets !   # bool is Label("bool") — wrong type for this use
```

See the [language reference](../README.md#stack-substitution-n--name) for a
full explanation of the `$name` syntax.

## Example

```
(42) [$int] typed_rets !
# stack: Substack { body: [42], args: None, ret: Some([Int]) }

fn ! [$bool] typed_rets !
# stack: Type(Fn { args: None, ret: Some([Bool]) })
```

## Errors

- Panics if the list contains non-type values.
- Panics if the target is not a `Substack`, `Script`, or `Fn` type.

## See also

- [typed_args.md](typed_args.md)
- [untyped_rets.md](untyped_rets.md)
- [fn.md](fn.md)
