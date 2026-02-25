To be able to run any significant code we need:
- structs
- match
- Reusable functions


## Structs:

A dictionary where all keys are labels.

Performance should be optimised, but that is a future issue.


## Match:

Stupidest solution is a built-in function that uses a dictionary. Assuming
sane size of dict type analysis could be done iteratively on the keys when
that becomes a thing, and the special label default can be used as key to set
what to do when no other key matches.

At that point, maybe the dictionary itself should return the value for the
default key when an unmatched key is given? If so a match invocation could be:

    # Dictionary used to match over
    {
      default: ("Hello" print),
      formal: ("Good day" print),
    }
    # The key, which isn't present in the dict
    informal
    # Function for getting a value from a dictionary
    # (returns a substack/tuple: `(dict value)` )
    dict_get
    # Invoke once to get the substack and invoke again to put it on the stack
    !!
    # The found value has now replaced the key on the stack
    # As we know that it is a substack that we wish to run, invoke
    !


## Reusable functions:

While supported by the define builtin, they could be badly created by defining
a global dict of functions... But with no syntactic sugar to easily access
entries in a dict it wouldn't work that well.
