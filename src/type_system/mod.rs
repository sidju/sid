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
/// allowing the cycle `SidType → DataValue → DataValue::Type → SidType` to
/// terminate at the `Box` boundary.
///
/// `Struct` and `Union` are not separate variants — they are expressed via
/// `Literal`:
/// - A union `{int, str}` is `Literal(Set([Type(Int), Type(Str)]))`.
/// - A struct type `{x: float, y: float}` is
///   `Literal(Struct([("x", Type(Float)), ("y", Type(Float))]))`.
/// This works because `Literal` dispatches on the inner `DataValue` kind, and
/// a `DataValue::Type(t)` element delegates matching to `t`.
#[derive(Debug, Clone, PartialEq)]
pub enum SidType {
    // Primitive types (pre-defined labels in the `types` namespace)
    Bool,
    Int,
    Float,
    Char,
    Str,
    Label,

    // Parametric container types (RPN: push type args then call constructor)
    /// `T list` — a homogeneous list whose elements are of type `T`
    List(Box<Self>),
    /// `K V map` — a homogeneous map with key type `K` and value type `V`
    Map {
        key: Box<Self>,
        value: Box<Self>,
    },

    /// `None` for args or ret means that dimension is not checked.
    /// Use `fn` built-in to produce `Fn { args: None, ret: None }` (any callable).
    /// Use `typed_args`/`typed_rets` on a substack to attach a signature.
    Fn {
        args: Option<Vec<Self>>,
        ret: Option<Vec<Self>>,
    },

    // C interop
    /// `T ptr` — a pointer to a value of type `T`; `SidType::Any` for `void*`
    Pointer(Box<Self>),

    // Combinators (RPN: push args then call constructor with @!)
    /// `base constraint require @!` — value must match both `base` AND `constraint`.
    Require {
        base: Box<Self>,
        constraint: Box<Self>,
    },
    /// `base forbidden exclude @!` — value must match `base` AND NOT match `forbidden`.
    Exclude {
        base: Box<Self>,
        forbidden: Box<Self>,
    },

    // Special
    /// Accepts any value; equivalent to a top type
    Any,
    /// Accepts any concrete (non-type) value — matches everything except `DataValue::Type(_)`
    Value,
    /// A specific value used in a type position.
    ///
    /// Matching dispatches on the kind of the inner `DataValue`:
    /// - `Type(t)` — delegates to `t.matches(value)` (enables nested type checks).
    /// - `List` — tuple type: positional match of each element as a sub-type.
    /// - `Set` — enum / union: value must match at least one element.
    /// - `Map` — structural: named fields must exist and match; extra fields ok.
    /// - `Struct` — ordered structural: same field order, no extra fields.
    ///   Also matches a `DataValue::List` of the same length (tuple equivalence).
    /// - Anything else — exact equality.
    Literal(Box<DataValue>),
}

impl SidType {
    /// Returns `true` if `value` is an instance of this type.
    ///
    /// - `List(T)` — every element must match `T`.
    /// - `Map { key: K, value: V }` — every (k, v) entry must match `K` and `V`.
    /// - `Fn { .. }` — value is a callable with matching type annotations.
    ///   `None` on a dimension is unconstrained; `Some` requires a matching
    ///   annotation set via `typed_args` / `typed_rets`.
    /// - `Literal(v)` — see variant doc for dispatch rules.
    pub fn matches(&self, value: &DataValue) -> bool {
        match self {
            SidType::Any => true,
            SidType::Value => !matches!(value, DataValue::Type(_)),
            SidType::Bool => matches!(value, DataValue::Bool(_)),
            SidType::Int => matches!(value, DataValue::Int(_)),
            SidType::Float => matches!(value, DataValue::Float(_)),
            SidType::Char => matches!(value, DataValue::Char(_)),
            SidType::Str => matches!(value, DataValue::Str(_)),
            SidType::Label => matches!(value, DataValue::Label(_)),

            SidType::Literal(lit) => match lit.as_ref() {
                // Type value → delegate to the inner type (enables types nested in
                // struct/set/list literals to act as type checks, not equality checks).
                DataValue::Type(t) => t.matches(value),
                // List literal → tuple type: positional match of each element as a sub-type.
                DataValue::List(pat_items) => match value {
                    DataValue::List(val_items) => {
                        pat_items.len() == val_items.len()
                            && pat_items
                                .iter()
                                .zip(val_items)
                                .all(|(p, v)| SidType::Literal(Box::new(p.clone())).matches(v))
                    }
                    _ => false,
                },
                // Set literal → enum / union: value must match at least one element.
                DataValue::Set(pat_items) => pat_items
                    .iter()
                    .any(|p| SidType::Literal(Box::new(p.clone())).matches(value)),
                // Map literal → two dispatch paths based on key types:
                //
                // All-label-key Map ("struct pattern") → ordered structural match:
                //   same field count, same label names in same order, values match.
                //   Also accepts a DataValue::List of the same length (tuple equivalence).
                //
                // Mixed / non-label-key Map → key-value pattern match:
                //   every pattern key must appear in the value map and its value match.
                DataValue::Map(pat_entries) => {
                    let all_labels = pat_entries
                        .iter()
                        .all(|(k, _)| matches!(k, DataValue::Label(_)));
                    if all_labels {
                        // Struct / ordered named-field pattern.
                        match value {
                            DataValue::Map(val_entries) => {
                                let val_label_entries: Vec<_> = val_entries
                                    .iter()
                                    .filter(|(k, _)| matches!(k, DataValue::Label(_)))
                                    .collect();
                                pat_entries.len() == val_label_entries.len()
                                    && pat_entries.iter().zip(val_label_entries).all(
                                        |((pk, pv), (vk, vv))| {
                                            pk == vk
                                                && SidType::Literal(Box::new(pv.clone()))
                                                    .matches(vv)
                                        },
                                    )
                            }
                            DataValue::List(items) => {
                                items.len() == pat_entries.len()
                                    && pat_entries.iter().zip(items).all(|((_, pv), vv)| {
                                        SidType::Literal(Box::new(pv.clone())).matches(vv)
                                    })
                            }
                            _ => false,
                        }
                    } else {
                        // Heterogeneous map pattern: required key-value pairs must exist.
                        match value {
                            DataValue::Map(val_entries) => pat_entries.iter().all(|(pk, pv)| {
                                val_entries
                                    .iter()
                                    .find(|(vk, _)| vk == pk)
                                    .map_or(false, |(_, vv)| {
                                        SidType::Literal(Box::new(pv.clone())).matches(vv)
                                    })
                            }),
                            _ => false,
                        }
                    }
                }
                // Pointer literal → exact address match; pointee type is checked via
                // SidType::matches so that e.g. types.null (pointee_ty: Any) matches
                // any null pointer regardless of its declared pointee type.
                DataValue::Pointer {
                    addr: pat_addr,
                    pointee_ty: pat_pointee,
                } => match value {
                    DataValue::Pointer {
                        addr: val_addr,
                        pointee_ty: val_pointee,
                    } => {
                        val_addr == pat_addr
                            && pat_pointee.matches(&DataValue::Type(val_pointee.clone()))
                    }
                    _ => false,
                },
                // All other literals → exact equality.
                other => value == other,
            },

            SidType::List(elem_ty) => match value {
                DataValue::List(items) => items.iter().all(|v| elem_ty.matches(v)),
                _ => false,
            },

            SidType::Map {
                key: key_ty,
                value: val_ty,
            } => match value {
                DataValue::Map(entries) => entries
                    .iter()
                    .all(|(k, v)| key_ty.matches(k) && val_ty.matches(v)),
                _ => false,
            },

            // None for args or ret means that dimension is unconstrained — accepts any callable.
            // Some requires the substack to have that dimension typed and matching.
            SidType::Fn { args, ret } => match value {
                DataValue::Substack {
                    args: val_args,
                    ret: val_ret,
                    ..
                }
                | DataValue::Script {
                    args: val_args,
                    ret: val_ret,
                    ..
                } => {
                    // Extract type-only vec from the named args for comparison.
                    let val_arg_types: Option<Vec<SidType>> = val_args
                        .as_ref()
                        .map(|a| a.iter().map(|(_, t)| t.clone()).collect());
                    let check =
                        |want: &Option<Vec<SidType>>, got: &Option<Vec<SidType>>| match (want, got)
                        {
                            (None, _) => true,
                            (Some(_), None) => false,
                            (Some(want), Some(got)) => {
                                want.len() == got.len()
                                    && want.iter().zip(got).all(|(w, g)| w.matches_type(g))
                            }
                        };
                    check(args, &val_arg_types) && check(ret, val_ret)
                }
                _ => false,
            },

            SidType::Pointer(pointee_ty) => match value {
                DataValue::Pointer {
                    pointee_ty: vty, ..
                } => pointee_ty.matches_type(vty),
                _ => false,
            },

            SidType::Require { base, constraint } => {
                base.matches(value) && constraint.matches(value)
            }

            SidType::Exclude { base, forbidden } => {
                base.matches(value) && !forbidden.matches(value)
            }
        }
    }

    /// Returns `true` if `self` subsumes `other` as a type — i.e. every value
    /// that satisfies `other` also satisfies `self`. Used when checking type
    /// annotations against each other (e.g. fn_type arg/ret compatibility).
    ///
    /// `Any` subsumes everything. Otherwise exact structural equality is required.
    pub fn matches_type(&self, other: &SidType) -> bool {
        if matches!(self, SidType::Any) {
            return true;
        }
        self == other
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn substack(args: Option<Vec<(String, SidType)>>, ret: Option<Vec<SidType>>) -> DataValue {
        DataValue::Substack {
            body: vec![],
            args,
            ret,
        }
    }

    // ── primitive matches ─────────────────────────────────────────────────────

    #[test]
    fn any_matches_everything() {
        assert!(SidType::Any.matches(&DataValue::Int(1)));
        assert!(SidType::Any.matches(&DataValue::Bool(false)));
        assert!(SidType::Any.matches(&substack(None, None)));
    }

    #[test]
    fn primitive_matches_correct_variant() {
        assert!(SidType::Bool.matches(&DataValue::Bool(true)));
        assert!(!SidType::Bool.matches(&DataValue::Int(1)));
        assert!(SidType::Int.matches(&DataValue::Int(42)));
        assert!(!SidType::Int.matches(&DataValue::Bool(true)));
    }

    #[test]
    fn literal_matches_exact_value_only() {
        let lit = SidType::Literal(Box::new(DataValue::Int(7)));
        assert!(lit.matches(&DataValue::Int(7)));
        assert!(!lit.matches(&DataValue::Int(8)));
        assert!(!lit.matches(&DataValue::Bool(true)));
    }

    #[test]
    fn literal_type_delegates_to_inner_type() {
        let lit = SidType::Literal(Box::new(DataValue::Type(SidType::Int)));
        assert!(lit.matches(&DataValue::Int(42)));
        assert!(!lit.matches(&DataValue::Bool(true)));
    }

    #[test]
    fn literal_list_is_tuple_type() {
        let ty = SidType::Literal(Box::new(DataValue::List(vec![
            DataValue::Type(SidType::Int),
            DataValue::Type(SidType::Bool),
        ])));
        assert!(ty.matches(&DataValue::List(vec![
            DataValue::Int(1),
            DataValue::Bool(true)
        ])));
        assert!(!ty.matches(&DataValue::List(vec![DataValue::Int(1)])));
        assert!(!ty.matches(&DataValue::List(vec![
            DataValue::Bool(true),
            DataValue::Int(1)
        ])));
    }

    #[test]
    fn literal_map_label_keys_is_ordered_structural() {
        // All-label-key Map pattern: strict ordered match (same count, same order).
        let ty = SidType::Literal(Box::new(DataValue::Map(vec![(
            DataValue::Label("x".to_owned()),
            DataValue::Type(SidType::Int),
        )])));
        assert!(ty.matches(&DataValue::Map(vec![(
            DataValue::Label("x".to_owned()),
            DataValue::Int(1)
        ),])));
        assert!(!ty.matches(&DataValue::Map(vec![
            (DataValue::Label("x".to_owned()), DataValue::Int(1)),
            (DataValue::Label("y".to_owned()), DataValue::Int(2)), // extra not allowed
        ])));
        assert!(!ty.matches(&DataValue::Map(vec![
            (DataValue::Label("y".to_owned()), DataValue::Int(2)), // wrong field
        ])));
        assert!(!ty.matches(&DataValue::Map(vec![
            (DataValue::Label("x".to_owned()), DataValue::Bool(true)), // wrong type
        ])));
    }

    #[test]
    fn literal_map_non_label_keys_is_structural() {
        // Non-label-key Map pattern: structural match (pattern entries must be present, extras ok).
        let ty = SidType::Literal(Box::new(DataValue::Map(vec![(
            DataValue::Int(1),
            DataValue::Type(SidType::Bool),
        )])));
        assert!(ty.matches(&DataValue::Map(vec![
            (DataValue::Int(1), DataValue::Bool(true)),
            (DataValue::Int(2), DataValue::Int(99)), // extra ok
        ])));
        assert!(!ty.matches(&DataValue::Map(vec![
            (DataValue::Int(2), DataValue::Bool(true)), // key 1 missing
        ])));
    }

    // ── union (now expressed as Literal(Set of Type values)) ──────────────────

    #[test]
    fn union_matches_any_member() {
        let u = SidType::Literal(Box::new(DataValue::Set(vec![
            DataValue::Type(SidType::Int),
            DataValue::Type(SidType::Bool),
        ])));
        assert!(u.matches(&DataValue::Int(1)));
        assert!(u.matches(&DataValue::Bool(false)));
        assert!(!u.matches(&DataValue::Float(1.0)));
    }

    #[test]
    fn union_of_literals_matches_exact_values() {
        let u = SidType::Literal(Box::new(DataValue::Set(vec![
            DataValue::Int(1),
            DataValue::Int(2),
        ])));
        assert!(u.matches(&DataValue::Int(1)));
        assert!(u.matches(&DataValue::Int(2)));
        assert!(!u.matches(&DataValue::Int(3)));
    }

    // ── list ──────────────────────────────────────────────────────────────────

    #[test]
    fn list_matches_homogeneous_elements() {
        let ty = SidType::List(Box::new(SidType::Int));
        assert!(ty.matches(&DataValue::List(vec![DataValue::Int(1), DataValue::Int(2)])));
        assert!(!ty.matches(&DataValue::List(vec![
            DataValue::Int(1),
            DataValue::Bool(true)
        ])));
        assert!(ty.matches(&DataValue::List(vec![]))); // empty list always matches
    }

    // ── struct (label-keyed Map with Type field values) ───────────────────────

    fn struct_type(fields: &[(&str, SidType)]) -> SidType {
        SidType::Literal(Box::new(DataValue::Map(
            fields
                .iter()
                .map(|(n, t)| (DataValue::Label(n.to_string()), DataValue::Type(t.clone())))
                .collect(),
        )))
    }

    fn struct_val(fields: &[(&str, DataValue)]) -> DataValue {
        DataValue::Map(
            fields
                .iter()
                .map(|(n, v)| (DataValue::Label(n.to_string()), v.clone()))
                .collect(),
        )
    }

    #[test]
    fn struct_matches_exact_fields_in_order() {
        let ty = struct_type(&[("x", SidType::Int), ("y", SidType::Bool)]);
        assert!(ty.matches(&struct_val(&[
            ("x", DataValue::Int(1)),
            ("y", DataValue::Bool(true))
        ])));
        assert!(!ty.matches(&struct_val(&[
            ("x", DataValue::Int(1)),
            ("y", DataValue::Bool(true)),
            ("z", DataValue::Int(2)), // extra field not allowed
        ])));
        assert!(!ty.matches(&struct_val(&[
            ("y", DataValue::Bool(true)), // wrong order
            ("x", DataValue::Int(1)),
        ])));
        assert!(!ty.matches(&struct_val(&[
            ("x", DataValue::Bool(false)), // wrong type
            ("y", DataValue::Bool(true)),
        ])));
        assert!(!ty.matches(&struct_val(&[
      ("x", DataValue::Int(1)), // missing field
    ])));
    }

    #[test]
    fn struct_matches_list_positionally() {
        let ty = struct_type(&[("x", SidType::Int), ("y", SidType::Bool)]);
        assert!(ty.matches(&DataValue::List(vec![
            DataValue::Int(1),
            DataValue::Bool(true)
        ])));
        assert!(!ty.matches(&DataValue::List(vec![DataValue::Int(1)])));
        assert!(!ty.matches(&DataValue::List(vec![
            DataValue::Bool(true),
            DataValue::Int(1)
        ])));
    }

    // ── Fn ────────────────────────────────────────────────────────────────────

    #[test]
    fn fn_unconstrained_accepts_any_callable() {
        let ty = SidType::Fn {
            args: None,
            ret: None,
        };
        assert!(ty.matches(&substack(None, None)));
        assert!(ty.matches(&substack(Some(vec![("a".to_owned(), SidType::Int)]), None)));
        assert!(ty.matches(&substack(None, Some(vec![SidType::Bool]))));
        assert!(!ty.matches(&DataValue::Int(1)));
    }

    #[test]
    fn fn_typed_args_rejects_untyped_substack() {
        let ty = SidType::Fn {
            args: Some(vec![SidType::Int]),
            ret: None,
        };
        assert!(!ty.matches(&substack(None, None)));
    }

    #[test]
    fn fn_typed_args_accepts_matching_substack() {
        let ty = SidType::Fn {
            args: Some(vec![SidType::Int]),
            ret: None,
        };
        assert!(ty.matches(&substack(Some(vec![("a".to_owned(), SidType::Int)]), None)));
        assert!(!ty.matches(&substack(Some(vec![("a".to_owned(), SidType::Bool)]), None)));
    }

    #[test]
    fn fn_any_args_accepts_any_typed_args() {
        let ty = SidType::Fn {
            args: Some(vec![SidType::Any]),
            ret: None,
        };
        assert!(ty.matches(&substack(Some(vec![("a".to_owned(), SidType::Int)]), None)));
        assert!(ty.matches(&substack(Some(vec![("a".to_owned(), SidType::Str)]), None)));
        assert!(!ty.matches(&substack(None, None))); // untyped still rejected
    }

    #[test]
    fn fn_rejects_wrong_arg_count() {
        let ty = SidType::Fn {
            args: Some(vec![SidType::Int, SidType::Bool]),
            ret: None,
        };
        assert!(!ty.matches(&substack(Some(vec![("a".to_owned(), SidType::Int)]), None)));
    }

    #[test]
    fn fn_unconstrained_ret_accepts_typed_ret() {
        let ty = SidType::Fn {
            args: None,
            ret: None,
        };
        assert!(ty.matches(&substack(None, Some(vec![SidType::Int]))));
    }

    #[test]
    fn fn_typed_ret_rejects_untyped() {
        let ty = SidType::Fn {
            args: None,
            ret: Some(vec![SidType::Int]),
        };
        assert!(!ty.matches(&substack(None, None)));
        assert!(ty.matches(&substack(None, Some(vec![SidType::Int]))));
    }

    // ── matches_type ──────────────────────────────────────────────────────────

    #[test]
    fn any_subsumes_all_types() {
        assert!(SidType::Any.matches_type(&SidType::Int));
        assert!(SidType::Any.matches_type(&SidType::Bool));
        assert!(SidType::Any.matches_type(&SidType::Any));
    }

    #[test]
    fn non_any_requires_equality() {
        assert!(SidType::Int.matches_type(&SidType::Int));
        assert!(!SidType::Int.matches_type(&SidType::Bool));
    }
}
