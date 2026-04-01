# Deferred Computation

Substacks are first-class values — store them, pass them around, invoke them
later. This enables code-as-data patterns like callbacks, lazy evaluation, and
incremental operation building.

## Storing and invoking later

```
square (clone! 2 *!) global @!

3 square! square!   # define once, invoke many times
```

## Passing substacks as arguments

Built-ins like `while_do`, `do_while`, and `match` accept substacks as
arguments and invoke them at the appropriate time:

```
0 (10 less_than!) (1 add!) while_do!
```

The condition and body substacks are stored as data and executed repeatedly
by the loop machinery.

## Building operations incrementally

Compose substacks by constructing them at runtime:

```
op1 (2 multiply!) local!
op2 (-1 add!) local!


pipeline ($op1! $op2!) local!
pipeline !    # executes op1, then op2
```

Or select branches dynamically:

```
true {true: (op_a! op_b!), false: (op_c!)} match !
```

## Callback pattern

Pass a substack to another substack that invokes it when ready:

```
process
# define accepted arguments
{
  # Undefined data
  data: $types.Any,
  # A type accepting no arguments and returning nothing
  callback: fn {:} typed_args! [] typed_rets!
} <
  # A significant operation on the data
  #...
  callback!
> global @!
```

Not expected to be very useful due to the functional style, but syntactically
possible.
