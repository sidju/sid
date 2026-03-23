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

    /// `None` for args or ret means that dimension is not checked.
    /// Use `fn` built-in to produce `Fn { args: None, ret: None }` (any callable).
    /// Use `typed_arguments`/`typed_returns` on a substack to attach a signature.
    Fn { args: Option<Vec<Self>>, ret: Option<Vec<Self>> },

    // C interop
    /// `T ptr` — a pointer to a value of type `T`; `SidType::Any` for `void*`
    Pointer(Box<Self>),

    // Special
    /// Accepts any value; equivalent to a top type
    Any,
    /// A specific value used in a type position, e.g. `"yes"` in `{"yes", "no"}`
    Literal(Box<DataValue>),
}

impl SidType {
  /// Returns `true` if `value` is an instance of this type.
  ///
  /// Structural types recurse into their contents:
  /// - `List(T)` — every element must match `T`.
  /// - `Map { key: K, value: V }` — every entry's key must match `K` and value `V`.
  /// - `Struct(fields)` — the value must be a Struct with each named field
  ///   present and matching the declared type. Extra fields are allowed.
  /// - `Union(types)` — at least one member must match.
  /// - `Fn { .. }` — checks that the value is a callable (Substack or Script);
  ///   argument/return types are not verified at runtime because substacks carry
  ///   no type annotations. Use `Any` for both to match any callable.
  pub fn matches(&self, value: &DataValue) -> bool {
    match self {
      SidType::Any          => true,
      SidType::Bool         => matches!(value, DataValue::Bool(_)),
      SidType::Int          => matches!(value, DataValue::Int(_)),
      SidType::Float        => matches!(value, DataValue::Float(_)),
      SidType::Char         => matches!(value, DataValue::Char(_)),
      SidType::Str          => matches!(value, DataValue::Str(_)),
      SidType::Label        => matches!(value, DataValue::Label(_)),
      SidType::Literal(lit) => value == lit.as_ref(),
      SidType::Union(types) => types.iter().any(|t| t.matches(value)),

      SidType::List(elem_ty) => match value {
        DataValue::List(items) => items.iter().all(|v| elem_ty.matches(v)),
        _ => false,
      },

      SidType::Map { key: key_ty, value: val_ty } => match value {
        DataValue::Map(entries) =>
          entries.iter().all(|(k, v)| key_ty.matches(k) && val_ty.matches(v)),
        _ => false,
      },

      SidType::Struct(fields) => match value {
        DataValue::Struct(struct_fields) => fields.iter().all(|(name, field_ty)| {
          struct_fields.iter()
            .find(|(n, _)| n == name)
            .map_or(false, |(_, v)| field_ty.matches(v))
        }),
        _ => false,
      },

      // None for args or ret means that dimension is unconstrained — accepts any callable.
      // Some requires the substack to have that dimension typed and matching.
      SidType::Fn { args, ret } => match value {
        DataValue::Substack { args: val_args, ret: val_ret, .. } |
        DataValue::Script   { args: val_args, ret: val_ret, .. } => {
          let check = |want: &Option<Vec<SidType>>, got: &Option<Vec<SidType>>| match (want, got) {
            (None, _)                    => true,
            (Some(_), None)              => false,
            (Some(want), Some(got)) =>
              want.len() == got.len()
              && want.iter().zip(got).all(|(w, g)| w.matches_type(g)),
          };
          check(args, val_args) && check(ret, val_ret)
        },
        _ => false,
      },

      SidType::Pointer(pointee_ty) => match value {
        DataValue::Pointer { pointee_ty: vty, .. } => pointee_ty.matches_type(vty),
        _ => false,
      },
    }
  }

  /// Returns `true` if `self` subsumes `other` as a type — i.e. every value
  /// that satisfies `other` also satisfies `self`. Used when checking type
  /// annotations against each other (e.g. fn_type arg/ret compatibility).
  ///
  /// `Any` subsumes everything. Otherwise exact structural equality is required.
  pub fn matches_type(&self, other: &SidType) -> bool {
    if matches!(self, SidType::Any) { return true; }
    self == other
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn substack(args: Option<Vec<SidType>>, ret: Option<Vec<SidType>>) -> DataValue {
    DataValue::Substack { body: vec![], args, ret }
  }

  // ── primitive matches ─────────────────────────────────────────────────────

  #[test] fn any_matches_everything() {
    assert!(SidType::Any.matches(&DataValue::Int(1)));
    assert!(SidType::Any.matches(&DataValue::Bool(false)));
    assert!(SidType::Any.matches(&substack(None, None)));
  }

  #[test] fn primitive_matches_correct_variant() {
    assert!( SidType::Bool.matches(&DataValue::Bool(true)));
    assert!(!SidType::Bool.matches(&DataValue::Int(1)));
    assert!( SidType::Int.matches(&DataValue::Int(42)));
    assert!(!SidType::Int.matches(&DataValue::Bool(true)));
  }

  #[test] fn literal_matches_exact_value_only() {
    let lit = SidType::Literal(Box::new(DataValue::Int(7)));
    assert!( lit.matches(&DataValue::Int(7)));
    assert!(!lit.matches(&DataValue::Int(8)));
    assert!(!lit.matches(&DataValue::Bool(true)));
  }

  // ── union ─────────────────────────────────────────────────────────────────

  #[test] fn union_matches_any_member() {
    let u = SidType::Union(vec![SidType::Int, SidType::Bool]);
    assert!( u.matches(&DataValue::Int(1)));
    assert!( u.matches(&DataValue::Bool(false)));
    assert!(!u.matches(&DataValue::Float(1.0)));
  }

  // ── list ──────────────────────────────────────────────────────────────────

  #[test] fn list_matches_homogeneous_elements() {
    let ty = SidType::List(Box::new(SidType::Int));
    assert!( ty.matches(&DataValue::List(vec![DataValue::Int(1), DataValue::Int(2)])));
    assert!(!ty.matches(&DataValue::List(vec![DataValue::Int(1), DataValue::Bool(true)])));
    assert!( ty.matches(&DataValue::List(vec![])));  // empty list always matches
  }

  // ── struct ────────────────────────────────────────────────────────────────

  #[test] fn struct_matches_required_fields() {
    let ty = SidType::Struct(vec![("x".to_owned(), SidType::Int)]);
    assert!( ty.matches(&DataValue::Struct(vec![
      ("x".to_owned(), DataValue::Int(1)),
      ("y".to_owned(), DataValue::Bool(true)), // extra field allowed
    ])));
    assert!(!ty.matches(&DataValue::Struct(vec![
      ("x".to_owned(), DataValue::Bool(false)), // wrong type
    ])));
    assert!(!ty.matches(&DataValue::Struct(vec![]))); // missing field
  }

  // ── Fn ────────────────────────────────────────────────────────────────────

  #[test] fn fn_unconstrained_accepts_any_callable() {
    let ty = SidType::Fn { args: None, ret: None };
    assert!( ty.matches(&substack(None, None)));
    assert!( ty.matches(&substack(Some(vec![SidType::Int]), None)));
    assert!( ty.matches(&substack(None, Some(vec![SidType::Bool]))));
    assert!(!ty.matches(&DataValue::Int(1)));
  }

  #[test] fn fn_typed_args_rejects_untyped_substack() {
    let ty = SidType::Fn { args: Some(vec![SidType::Int]), ret: None };
    assert!(!ty.matches(&substack(None, None)));
  }

  #[test] fn fn_typed_args_accepts_matching_substack() {
    let ty = SidType::Fn { args: Some(vec![SidType::Int]), ret: None };
    assert!( ty.matches(&substack(Some(vec![SidType::Int]), None)));
    assert!(!ty.matches(&substack(Some(vec![SidType::Bool]), None)));
  }

  #[test] fn fn_any_args_accepts_any_typed_args() {
    let ty = SidType::Fn { args: Some(vec![SidType::Any]), ret: None };
    assert!( ty.matches(&substack(Some(vec![SidType::Int]), None)));
    assert!( ty.matches(&substack(Some(vec![SidType::Str]), None)));
    assert!(!ty.matches(&substack(None, None))); // untyped still rejected
  }

  #[test] fn fn_rejects_wrong_arg_count() {
    let ty = SidType::Fn { args: Some(vec![SidType::Int, SidType::Bool]), ret: None };
    assert!(!ty.matches(&substack(Some(vec![SidType::Int]), None)));
  }

  #[test] fn fn_unconstrained_ret_accepts_typed_ret() {
    let ty = SidType::Fn { args: None, ret: None };
    assert!( ty.matches(&substack(None, Some(vec![SidType::Int]))));
  }

  #[test] fn fn_typed_ret_rejects_untyped() {
    let ty = SidType::Fn { args: None, ret: Some(vec![SidType::Int]) };
    assert!(!ty.matches(&substack(None, None)));
    assert!( ty.matches(&substack(None, Some(vec![SidType::Int]))));
  }

  // ── matches_type ──────────────────────────────────────────────────────────

  #[test] fn any_subsumes_all_types() {
    assert!(SidType::Any.matches_type(&SidType::Int));
    assert!(SidType::Any.matches_type(&SidType::Bool));
    assert!(SidType::Any.matches_type(&SidType::Any));
  }

  #[test] fn non_any_requires_equality() {
    assert!( SidType::Int.matches_type(&SidType::Int));
    assert!(!SidType::Int.matches_type(&SidType::Bool));
  }
}
