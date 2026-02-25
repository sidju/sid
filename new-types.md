# Alternate typing approaches:

## C compatible:

(Since the possible value set concept is a pain to implement)

Just match C types exactly, with some additional structs for data structures and
tagged unions.

Types would be:
- int, uint & 8, 16, 32, 64, size bits
- char, grapheme, string
- list, set, map types
- struct, tagged unions
- function

## Screw it, no typing:

Could be decently reasonable in the interim, but makes defining functions quite
hopeless.

Would also require syntax changes later.

# Semi related, match block options:

- Go the golang way, make it a syntactically handy way to do if elif else blocks
- Possibly create a separate match system for tagged union / enums that can
  require completeness.
