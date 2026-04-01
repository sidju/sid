# Built-in Reference

All built-in functions, grouped by availability.

## Comptime + Runtime

| Built-in        | Summary |
|-----------------|---------|
| [clone](clone.md) | Duplicate the top stack value |
| [drop](drop.md) | Discard the top stack value |
| [eq](eq.md) | Structural equality comparison |
| [not](not.md) | Boolean negation |
| [assert](assert.md) | Panic if top value is `false` |
| [null](null.md) | Push a null pointer |
| [ptr_cast](ptr_cast.md) | Re-type a pointer's pointee type |
| [debug_stack](debug_stack.md) | Print top N stack items to stderr |
| [load_scope](load_scope.md) | Unpack a struct into global scope |
| [load_local](load_local.md) | Unpack a struct into local scope |
| [local](local.md) | Bind a value to a name in local scope |
| [get](get.md) | Look up a label in local → global → builtins |
| [get_local](get_local.md) | Look up a label in local scope only |
| [get_global](get_global.md) | Look up a label in global scope only |
| [c_load_header](c_load_header.md) | Parse a C header into function signatures |
| [fn](fn.md) | Push an unconstrained callable type |
| [typed_args](typed_args.md) | Set the args type annotation on a callable |
| [typed_rets](typed_rets.md) | Set the ret type annotation on a callable |
| [untyped_args](untyped_args.md) | Clear the args type annotation on a callable |
| [untyped_rets](untyped_rets.md) | Clear the ret type annotation on a callable |

## Runtime Only

| Built-in        | Summary |
|-----------------|---------|
| [while_do](while_do.md) | Loop while condition is true (check-first) |
| [do_while](do_while.md) | Loop while condition is true (run-first) |
| [match](match.md) | Pattern-match a value against cases |
| [c_link_lib](c_link_lib.md) | Resolve C function signatures against a shared library |
| [ptr_read_cstr](ptr_read_cstr.md) | Read a null-terminated C string from a pointer |
