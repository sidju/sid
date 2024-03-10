# Execution concept for stack-based language concept

## Stacks

One of each stack exists for each thread of execution. They are created and
merged as threads are forked and joined, in a way that should give the same
resulting behaviour as if it was all evaluated in a single sequential thread.

### Data stack(s):

Is empty at start of execution and holds the application state.

### Program stack(s):

Would hold the reverse polish syntax in a binary representation. Is put onto the
data stack and interpreted upon it.

# Runtime concept for imperative style language concept

TODO
