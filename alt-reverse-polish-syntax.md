# Syntax plan:

Alternate reverse Polish Notation based functional syntax

Treats functions as value first and function second, since that makes the
handling of functions more consistent with the rest of the language.

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
`'a'`, `'👮‍♀️'`, `'8'` and such.

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
`{one: "one", two: 2.0, three: 3}` and such.

Identified by enclosure by `{}` and `:` in the first parsing level inside.
Will return error if there isn't a key for every value inside
(`nr : >= nr ,` at every character of parsing).

Held types not required to fully match as that kind of validation should be
handled by the type system?

### substack:
`(16 16 mul sqrt sqrt)`

Identified by enclosure by `()`.

Creates a value that can be executed via `!`.

It does have input types if the functions invoked within take more arguments
than are enclosed. Likewise it has output types matching the values left on
the substack after invocation.

### script:
`<"Hi there" print "handsome" print>`

Identified by enclosure by `<>`.

Creates a value that can be executed via `!`.

Same input/output type logic as substack, but internal execution is guaranteed
to be sequential. (Intended use is to order calls to functions with
side-effects.)

## Examples:

### Declare a value to scope:
Assuming `def` has argument-decl `{name: str, value: Any}`:

    "approx_pi" 3 def!

### A match case:
Assuming `match` has argument-decl
`{value: Any, cases: [{case: Type, action: Substack | Script}]}` an invocation
could be:

    "Yes" [
      { case: {"yes","Yes","y", "Y"}, action: (true) },
      { case: Any, action: <"That's not a yes" print! false> }
    ] match!

### Declare a function to scope:
Assuming `def` has argument-decl `{name: str, value: Any}` and `fn` takes
argument-decl

    {
      description: str,
      # The field names are mainly documentation, the order and types are what's
      # used to take values from the stack.
      argument-decl: AnyStruct,
      body: Script | Substack,
      return-decl: Any,
    }

the creation of a function to print the same message twice looks like this:

    "print_twice"
      "Prints given message to stdout twice"
      { message: str } <
        # needed since each print will consume one value from the stack
        duplicate!
          # Indented relative the data/source of the data it consumes
          print!
          print!
      >
      fn!
    def!

(Formatting praxis is highly debatable.)

Executing this function should be functionally equivalent to running the script
`<duplicate! print! print!>`. The benefit in creating functions is in adding a
layer of type validation and documentation around the code, aiding the developer
with more local type errors and function descriptions accessible from debugger.
