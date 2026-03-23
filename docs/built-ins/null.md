# `null`

**Availability:** comptime + runtime

Pushes a null pointer (`Pointer { addr: 0, pointee_ty: Any }`). Useful as a
sentinel or for C FFI calls that accept a nullable pointer.

## Stack effect

```
...  →  ... Pointer(0)
```

## Example

```
null !   # stack: Pointer { addr: 0, pointee_ty: Any }
```
