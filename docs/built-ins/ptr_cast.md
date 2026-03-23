# `ptr_cast`

**Availability:** comptime + runtime

Pops a `Pointer` and a `Type`, and returns the same pointer with its
`pointee_ty` replaced by the given type. Does not affect the address.

## Stack effect

```
... Pointer Type  →  ... Pointer
```

`Type` is on top; `Pointer` is below it.

## Example

```
null ! [$int] ptr_cast !
# stack: Pointer { addr: 0, pointee_ty: Int }
```

## Errors

- Panics if the top value is not a `Type`.
- Panics if the second value is not a `Pointer`.
