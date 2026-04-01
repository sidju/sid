# SID Language Reference

SID is a stack-based language using Reverse Polish Notation (RPN). Values are
pushed onto a data stack; functions consume values from the top and push their
results back.

Notably SID only really has three types of stack values:

| Value Type | Stack effect                                                     |
| ---------- | ---------------------------------------------------------------- |
| Literals   | Simply added on top of the data stack.                           |
| Templates  | May consume stack entries to render, then put on the data stack. |
| Invokes    | Queues function on the data stack onto the program stack.        |

---

## Documentation Index

| Document | Covers |
| -------- | ------ |
| [Syntax](syntax.md) | Literals, templates, nested templates, `$` substitution, nested invokes, comments |
| [Execution](execution.md) | Invoke (`!`), comptime (`@!`), execution pipeline, scopes |
| [Types](types.md) | Labels, primitive types, container types, function types |
| [Built-ins](built-ins/) | Reference for every built-in function |
