# `untyped_rets`

**Availability:** comptime + runtime

Clears the `ret` type annotation on a substack, script, or `Fn` type value,
setting it back to `None` (unconstrained).

## Stack effect

```
... callable  →  ... callable
```

## Example

```
(42) [$int] typed_rets ! untyped_rets !
# stack: Substack { body: [42], args: None, ret: None }
```

## Errors

- Panics if the target is not a `Substack`, `Script`, or `Fn` type.

## See also

- [typed_rets.md](typed_rets.md)
- [untyped_args.md](untyped_args.md)
