use super::Value;

pub mod mock_side_effector;

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum SideEffectFunction {
  Print,
}

pub trait SideEffector {
  fn invoke(&mut self,
    function: SideEffectFunction,
    stack: &mut Vec<Value>,
  );
}
