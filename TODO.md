- Probably add `pointer_write`, `pointer_read` and `pointer_offset`, to enable the
  same basic pointer usage that C allows through base syntax `*(p+2)`.
- Consider how comptime should interact with render, should we have comptime render?
- Decide our attitude to `clone`, `drop`, `get`, and such built-ins that should be
  redundant with `($1 $1)!`, `($2)!`, `($label)!`. Should they exist?
- Add `global` built-in: pops `{name: label, value: Any}` and writes to global scope.
  Comptime only — prevents runtime write-order races in global scope.
- Add `swap` built-in: equivalent to `($2 $1)!` but more readable.
- Add `$n.field` syntax: access a named field on a map value at a stack position.
  For example `$1.x` would extract the `x` field from the topmost consumed map
  value, eliminating manual field extraction patterns.

## Static analysis (future)

- Validate that all arms of a `match` leave the same net stack change.
  Requires static stack-effect tracking rather than speculative execution.
  Deferred until a static analysis pass exists.
- Run a static type validation, ideally adding type-restrictions or other
  meta-types to perform the most detailed validation possible.

## Documentation and examples (next up)

- Document `matches` semantics for every type in DESIGN.md
- Add Structs/Maps section to DESIGN.md (unified Map type, label-key distinction,
  dot-label access, struct matching)
- Add Match section to DESIGN.md (`{pattern: action} match !`, type dispatch,
  enum dispatch, first-match-wins)
- Rewrite `fn` example in DESIGN.md (substack + `typed_args` + `typed_rets`)
- Verify `def` example in DESIGN.md is still valid
- Update `examples/fizz-buzz.sid`
- Update `examples/structs.sid` (`&` → `$`, use real `local!`, fix unimplemented syntax)
- Update `examples/reordering.sid` (`&n` → `$n`)
- Update `examples/nurbs.sid` (match syntax and other outdated constructs)

## `&n` back-reference operator

A non-destructive stack access analogous to `$n` (which moves a value). `&n`
would copy a value from the stack without consuming it, enabling struct field
reading and other patterns where the original must be preserved. The `&`
character is reserved for this purpose.

## `describe` annotation built-in

A built-in to attach a human-readable description string to any value.
Descriptions would be dropped in release/compiled builds but remain visible in
the interpreter and debugger. Useful for self-documenting functions and types.

## `typed_rets` struct auto-unpacking (future)

Allow `typed_rets` to accept a struct type in addition to a list of types.
When a struct type is given, return values would be automatically packaged into
a named struct on return, mirroring the auto-packing behaviour of `typed_args`.


