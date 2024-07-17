use std::collections::HashMap;

use crate::BuiltInFunction;

// use crate::{
//   RealValue,
//   ProgramValue,
//   DataValue,
// };


// Well this is obviously shit, but until later
pub fn get_built_in_functions() -> HashMap<&'static str, &'static dyn BuiltInFunction> {
  HashMap::new()
}