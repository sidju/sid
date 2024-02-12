# SID scopes:

There are only two scopes, local and global. 

## Global scope:

The global scope is the root of all source files and is accessible from
everywhere in the program. It can only be written to from the root of a source
file and due to the way it could otherwise cause race conditions evaluation in
the root of source files is strictly sequential and blocking.

## Local scope:

The local scope exists in every function and by default has definitions for
the function's arguments. If local scope has a definition overlapping with one
in global scope the local scope has priority.

## Writing to scope:

The `def` function can be used to define values into current scope. Defining
the same value into scope multiple times is only allowed if in a sequentially
executing scope (either the root scope or a function with a list of
operations).

In the long term usage of the defined value will be automatically registered
as a dependence and scheduling in set functions will ensure that the
definition is done before any usage of it.
