# `untyped_args`

**Availability:** comptime + runtime

Clears the `args` type annotation on a substack, script, or `Fn` type value,
setting it back to `None` (unconstrained).

## Stack effect

```
... callable  →  ... callable
```

## Example

```
(42) [$int] typed_args ! untyped_args !
# stack: Substack { body: [42], args: None, ret: None }
```

## Errors

- Panics if the target is not a `Substack`, `Script`, or `Fn` type.

## See also

- [typed_args.md](typed_args.md)
- [untyped_rets.md](untyped_rets.md)
