# (Not ruby code, it just had decent syntax highlighting for the syntax)

# Core concept: stack based parsing with reverse polish notation

# To enable defining functions and treating them as first-class-citizens
# a function, sub-stack or script is not executed if preceeded by `!`.

# Special function declares to scope
# Define a custom type. Types can be derived as described in types.md
"myint" int def
# Define a constant.
"approx_pi" 3 def

# Sub-stacks can be declared with parantheses
# Value references are converted to their value at definition of the sub-stack,
# but invocations within the substack are only processed when the sub-stack is
# itself invoked.
# A sub-stack doesn't guarantee execution order beyond pure causality.
# (Since these calls don't interdepend at all they may be executed in parallel
# or in any order.)
(
  "hi" print
  "there" print
)
# If you need sequential execution to order side effects correctly, you should
# declare a script using angle brackets instead
<
  "hi" print
  "there" print
>
# Sub stacks aren't a tool to create parallelism, it is a way to define code
# without running it. Parallelism is derived where possible by the compiler,
# except in scripts.
# Both sub-stacks and scripts are allowed to operate upon the stack where
# they are invoked and type validation will occur relative to that location.
# Both sub-stacks and scripts are invoked at declaration unless preceeded by a
# `!`.

# A proper function is created from a substack or script by type data to it
# <description> <argument-decl> <sub-stack/script> <return-decl> fn
# So defining a function becomes
# <fn-name> <description> <argument-decl> <body> <return-decl> fn def
# Declaring a function that takes a string, prints it twice and returns nothing
# (using a script body, since print is a side-effect function)
# (note the `!` before the script, so it is put on the stack as an argument
# instead of being executed)
"print_twice" "Prints given message to stdout twice" { message: str } !<
  # We need to duplicate, since each print consumes a string from the stack
  duplicate
    # print should be indented relative to the function whose output it uses
    print
    print
# No return value -> {}
> {} fn def

# Invoking print_twice is equivalent to invoking !<duplicate print print>
# The difference is that functions are type validated at construction, giving
# more local and specific errors to the developer, and provide some inherent
# documentation through its description and typing.


# Match cases are built from a list of
# { case: Type, action: Substack | Script | Function }
# the first case where the value is of the type in case is executed on the
# current stack
# So this prints "That's a yes!" and returns a true boolean:
"Yes" [
  { case: {"yes", "Yes", "y", "Y"}, action: !<"That's a yes!" print! true> }
  # Both branches need to put the same number of entries onto the stack, as the
  # compiler does optimisations based on knowing the nr of values on the stack.
  { case: Any, action: !<"That's not yes" print! false> }
] match



# Some literals:
# bool
false
true
# int
-3
# float
3.
# char (utf-8 grapheme cluster, size not predictable)
'👮‍♀️'
# string
"hello" 
# list
[foo, bar, baz]
# set
{foo, bar, baz}
# struct
{foo: fooval, bar: barval, baz: bazval}
# substack (can be considered a tuple)
("hello there" 54 sqrt! false)
# script
<"hello there" print!>

# As is common we don't have a literal syntax for dictionaries/maps, since they
# can be constructed from a list of structs easily enough and rarely need to be
# declared as literals.
