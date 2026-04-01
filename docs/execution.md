# Execution

## Invoke (`!`)

`!` pops the top of the stack and executes it. The preceding value must be
executable (substack, script, built-in, or C function) or a label resolving
to one.

```
(1 2 add!) !   # pushes the substack, then invokes it
clone!         # label resolves to the built-in, then invokes it
```

The shorthand `foo !` is so common it is written `foo!` throughout examples.
This shorthand also applies to comptime invoke: `foo @!` is written `foo@!`.

## Comptime Invoke (`@!`)

`@!` marks an invocation to be evaluated during the comptime pass, before
any code runs. The annotation is contagious: all `!` tokens inside a
`@!`-invoked body are also treated as comptime.

```
int list@!   # builds a List type at comptime
```

## Execution Pipeline

```
source text
  ↓  parse
Vec<TemplateValue>        # may contain $n / $name substitution slots
  ↓  comptime pass
Vec<TemplateValue>        # @! sites evaluated; must have concrete inputs
  ↓  render
Vec<ProgramValue>         # $n/$name resolved against parent stack/scope
  ↓  interpret
DataValue                 # written to the data stack
```

## Scopes

| Scope  | Accessible from | Writable from          |
|--------|-----------------|------------------------|
| Global | everywhere      | file root only         |
| Local  | inside a function | function arguments   |

Local scope shadows global for overlapping names.
