use super::Value;

pub enum BuiltInFunction {
  Print,
}

impl BuiltInFunction {
  pub fn execute(
    side_effector: &mut impl SideEffector,
    stack: &mut Vec<Value>,
  ) -> Result<(), Box<dyn Error>> {
    match self {
      Self::Print => side_effector.print(stack.pop())?,
    }
  }
}
