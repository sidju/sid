use std::error::Error;

use super::{
  Value,
};

pub mod side_effects;
use side_effects::SideEffectFunction;

pub fn resolve_label(
  label: &str,
) -> Result<Value, Box<dyn Error>> {
  // First we match against built in functions
  match label {
    "print" => Ok(SideEffectFunction::Print.into()),
    _ => panic!("Label not found"),
  }
}
