# SID typing:

The typing is not dynamic nor entirely static, it is set based and mainly geared
towards intelligent auto-conversion when a value has no explicit type.


## Type validation

Typing in SID is implicit for values and explicit for argumentss. It always
ensures that a value is valid for the way it is being used, called *Validity
typing*. It operates as a validation layer on top of the unbreakable memory
representation based root typing system.

Validity typing could be considered a separate dynamic type system for every
root type, (so it doesn't do any of the funky auto-conversions that often
cause issues). It only allows automatic no-op conversions where a value
shares its root type with the type it should be converted into and fulfils
the validity requirements of that type.

As an example division obviously cannot divide by zero, which is enforced by a
non-zero int type:
If called with a literal integer it will be converted to the non-zero int type
if valid. So `divide (dividend: 4, divisor: 2)` will evaluate to `2` and
`divide (dividend: 4, divisor: 0)` will give a type error since 0 isn't a valid
value for the non-zero type.

Trying to call the function with a string will also give a type error, as the
non-zero type is int based to begin with. (Rather than trying to interpret
the string as a number or something weird.)


## Base concept, what is a type:

The typing concept is based on sets of valid values. The base types are sets of
all representable values for the type and further types can be constructed in
two ways:

1. A set of values is a type that only allows those values or subset thereof.
2. A struct is a type allowing a struct where all the fields are subsets of the
   corresponding field in the type struct.

Way nr 1 should be considered the main way and can be done both by literal set
declaration `{1,2}`, which then behaves more or less like an enum, and via set
operations on existing types `set_sub (minuend: int, subtrahend: {0})`. Note
that a set can contain multiple different types, so for example `{true, "true"}`
is a valid type that can accept either a bool or a string. This is mostly useful
since matching is based on matching to types, as the resulting value won't be
usable without knowing what root type it is.

(Set operations are better suited for types with many valid states, as they
unlike literal sets don't need to be constructed and kept in memory.)


### Declaring meta-types; list, set and map:

Meta types need to be given an internal type to be a usable type specification.
This is done by running them as functions with the internal type as argument(s),
for example `list int` specifies a list containing integers, `set int` a set of
integers and `map (key: str, val: int)` specifies a map from str to int.

And of course you can provide a custom type specification for meta-types, just
provide a custom type for the type argument(s), `list (x: int, y: int)` is a
list of a struct matching (x: int, y: int).
