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

// mod restriction; // TODO: re-enable once TypeRestriction is fleshed out

use crate::DataValue;

/// A first-class type value in SID.
///
/// Types are values: any expression in a type position produces a `SidType`.
/// Container variants use `Box<Self>` or `Vec<Self>` so the enum is not
/// infinitely sized. `Literal` holds a concrete `DataValue` on the heap,
/// allowing the cycle `SidType → DataValue → RealValue → SidType` to
/// terminate at the `Box` boundary.
#[derive(Debug, Clone, PartialEq)]
pub enum SidType {
    // Primitive types (pre-defined labels in global scope)
    Bool,
    Int,
    Float,
    Char,
    Str,
    Label,

    // Parametric container types (RPN: push type args then call constructor)
    /// `T list` — a homogeneous list whose elements are of type `T`
    List(Box<Self>),
    /// `K V map` — a map with key type `K` and value type `V`
    Map { key: Box<Self>, value: Box<Self> },

    // Composite types built from set / struct literals
    /// `{T1, T2, …}` where every element is a type — a union of types
    Union(Vec<Self>),
    /// `{field1: T1, field2: T2, …}` where every value is a type — a struct type
    Struct(Vec<(String, Self)>),

    // Function / substack type (`{args: T, ret: T} fn_type!`)
    Fn { args: Box<Self>, ret: Box<Self> },

    // Special
    /// Accepts any value; equivalent to a top type
    Any,
    /// A specific value used in a type position, e.g. `"yes"` in `{"yes", "no"}`
    Literal(Box<DataValue>),
}
