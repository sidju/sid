# Syntax plan:

Reverse Polish Notation based functional syntax

## Literals:

### bool:
`true` or `false`

### int:
`-5`, `59`, and such.

Identified optional starting `-` followed by only base 10 numbers.

### float:
`.78`, `-50.93`, and such.

Identified by optional starting `-` followed by base 10 numbers and at least
one `.`.

### char:
`'a'`, `'👮‍♀️`', `'8'` and such.

Identified by enclosure by `'`, will error if contents aren't a valid unicode
grapheme. (Annoying job, but should be done for correctness.)

### string:
`"ghoul"`, `"bar"` and such.

Identified by enclosure by `"`, will error if contents aren't complete and
valid unicode graphemes.

Backing implementation should be bytewise, but functions should work on the
unicode graphemes when possible. 

### list:
`[1, "two", 3, '4']` and such.

Identified by enclosure by `[]`.

Held types not required to fully match as that kind of validation should be
handled by the type system?

### set:
`{1, "two", 3, '4'}` and such.

Identified by enclosure by `{}` and no `:` in the first parsing level inside.

Held types not required to fully match as that kind of validation should be
handled by the type system?

### struct:
`{1: "one", 2: "two", "two": 2, "one": 1}` and such.

Identified by enclosure by `{}` and `:` in the first parsing level inside.
Will return error if there isn't a key for every value inside
(`nr : >= nr ,` at every character of parsing).

Held types not required to fully match as that kind of validation should be
handled by the type system?

### substack:
`(16 16 mul sqrt sqrt)`

Identified by enclosure by `()`.

Is executed immediately after parsing unless preceeded by `!`.

Since it is executed and put on the stack unless preceeded by `!` it can be used
as a tuple.

It does have input types if the functions invoked within take more arguments
than are enclosed. Likewise it has output types matching the values left on
the substack after invocation.

### script:
`<"Hi there" print "handsome" print>`

Identified by enclosure by `<>`.

Is executed immediately after parsing unless preceeded by `!`.

Same input/output type logic as substack, but internal execution is guaranteed
to be sequential. (Intended use is to order calls to functions with
side-effects.)
