// The type system comes in three parts:
// - Real types, the usual suspects.
// - Restrictions within those types, such as setting max/min on an int.
//   The place where most work is intended to be done.
// - Meta operations that allow combining any types in several ways. These come
//   with a base performance cost, and if more than one real type is allowed by
//   a definition then it will need to be matched into its Real type before any
//   operation can be done upon it.

// The type system is intended to work in two parts:
// - Instances have their Real type as metadata stored with their value.
//   (This enables runtime match operations to identify if an instance is valid
//   within a type restriction)
// - Further restrictions are verified to be followed at compile time, with some
//   kind of match operation to further restrict and explicitly handle when that
//   further restriction wasn't followed.
// - Relaxing of restrictions should be detected at compile time and a noop in
//   the runtime.

mod restriction;
