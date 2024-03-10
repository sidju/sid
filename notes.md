# Wild ideas:

- Every function is a match case (as a way to define valid inputs).
  (Would then need an `invalid` action, that would error at compile time if
  reachable.)
  This could possibly act to remove the need for explicit function creation, as
  the match case would verify the input and a substack/script acts as the
  different function implementations depending on the input types.
- Strings vs. labels. Maybe add label values?! Then we could avoid using
  strings when defining labels, but it would be odd to refer to labels before
  they exist.
  (Or we could make labels default to themselves as strings, but that would be
  an huge source of bugs for a hundred years. It is better to error when a
  label is undefined.)
- Use map to define match cases. The key sets are thus required to be distinct.
  (With the set operations that should be built-in that shouldn't be too hard.)
  Probably superseeded by using list of case,action tuples.
      input [
        [{"yes", "Yes"}, (true)],
        [str, <"Taken as no" print! false>],
        [Any, <"Bad input string" print! false>],
      ] match!
- Lists should work as tuples.
  (Just do it like with structs!)


# Initial idea:

I hate varying numbers of arguments, and varying numbers of arguments per
function is "hard". So my initial idea was to provide structs as the only input
argument, and like that also force users to name their input arguments.
