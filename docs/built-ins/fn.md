# `fn`

**Availability:** comptime + runtime

Pushes an unconstrained callable type: `Fn { args: None, ret: None }`. This
is the base type for any substack or script, with no constraints on arguments
or return values.

Use `typed_args` and `typed_rets` to narrow the signature.

## Stack effect

```
...  →  ... Type(Fn { args: None, ret: None })
```

## Example

```
fn !
# stack: Type(Fn { args: None, ret: None })

fn ! [$int] typed_args ! [$bool] typed_rets !
# stack: Type(Fn { args: Some([Int]), ret: Some([Bool]) })
```

## See also

- [typed_args.md](typed_args.md)
- [typed_rets.md](typed_rets.md)
- [untyped_args.md](untyped_args.md)
- [untyped_rets.md](untyped_rets.md)
