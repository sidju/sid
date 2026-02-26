use std::collections::HashSet;
use crate::DataValue;

pub trait Restriction<T> {
  fn allows(&self, object: T) -> bool;
}

pub enum TypeRestriction {
  // Based on internal types
  // If you want specifically true or false you can use literal
  Bool,
  // Str and Char can only really be checked by base type and regex
  Str{min_len: Option<usize>, max_len: Option<usize>, regex: Option<String>},
  Char{regex: Option<String>},
  // Numbers have a lot of pretty easy ways to filter, but start simple
  Int{start: Option<i64>, end: Option<i64>},
  Float{start: Option<f64>, end: Option<f64>},

  // Substacks have very minimal type information right now, so we should add
  // input/output type information as functions to them automagically
  // Would match when:
  // - input/output in the restriction is of equal length to the corresponding
  //   field in the instance
  // - each entry in input of the restriction is a superset of the corresponding
  //   input entry in the instance
  // - each entry in the output of the restriction is a subset of the
  //   corresponding output entry in the instance
  Function{input: Vec<TypeRestriction>, output: Vec<TypeRestriction>},

  // Meta types
  // Lists are defined by the restriction placed upon their contents and length
  List{restriction: Box<Self>, min_len: Option<usize>, max_len: Option<usize>},

  // Generic/abstract
  Any, // Not recommended ever, but sure
  Literals(HashSet<DataValue>),
  Not(Box<Self>),
  Or(Box<Self>, Box<Self>),
  And(Box<Self>, Box<Self>),
  XOr(Box<Self>, Box<Self>),
  FunctionBased, // Todo, define how the check function is given
}

// The Self implementation returns true if all values allowed by the provided
// instance is a strict subset of the values allowed by the current instance.
// Returning false will also occur when it isn't possible to prove with the
// current code, in addition to when there is a real mismatch.
impl Restriction<&Self> for TypeRestriction {
  fn allows(&self, object: &Self) -> bool {
    use TypeRestriction::*;
    match (self, object) {
      // If self is Any always returns true
      (Any, _) => true,
      // The basic identical case
      (Bool, Bool) => true,
      // Types that support some restriction
      (
        Str{min_len: s, max_len: e, regex: r},
        Str{min_len: os, max_len: oe, regex: or}
      ) => {
        // For every optional restriction, if Some in self must be in other
        if let Some(s) = s {
          if let Some(os) = os {
            if os < s { return false; }
          }
          else { return false; }
        }
        if let Some(e) = e {
          if let Some(oe) = oe {
            if oe > e { return false; }
          }
          else { return false; }
        }
        if let Some(_) = r {
          return r == or;
        }
        true
      },
      (Char{regex: r}, Char{regex: or}) => {
        if let Some(_) = r {
          return r == or;
        }
        true
      },
      (Int{start: s, end: e}, Int{start: os, end: oe}) => {
        if let Some(s) = s {
          if let Some(os) = os {
            if os < s { return false; }
          }
          else { return false; }
        }
        if let Some(e) = e {
          if let Some(oe) = oe {
            if oe > e { return false; }
          }
          else { return false; }
        }
        true
      },
      (Float{start: s, end: e}, Float{start: os, end: oe}) => {
        if let Some(s) = s {
          if let Some(os) = os {
            if os < s { return false; }
          }
          else { return false; }
        }
        if let Some(e) = e {
          if let Some(oe) = oe {
            if oe > e { return false; }
          }
          else { return false; }
        }
        true
      },

      // Meta restrictions, that need to recurse

      // If no other comparison defined it is false
      _ => false
    }
  }
}
