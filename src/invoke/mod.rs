
use super::{
  Value,
  RealValue,
  Function,
};

pub mod side_effects;
use side_effects::SideEffectFunction;

pub fn resolve_label(
  label: &str,
) -> RealValue {
  // First we match against built in functions
  match label {
    "print" => SideEffectFunction::Print.into(),
    _ => panic!("Label not found"),
  }
}

pub fn realize_value(
  possibly_label: Value,
) -> RealValue {
  match possibly_label {
    Value::Label(x) => resolve_label(&x),
    Value::Real(x) => x,
  }
}

pub fn invoke(
  side_effector: &mut dyn side_effects::SideEffector,
  stack: &mut Vec<Value>,
) {
  match stack.pop().map(realize_value) {
    Some(RealValue::Fun(x)) => match x {
      Function::SideEffect(y) => {
        side_effector.invoke(y, stack)
      }
    }
    _ => panic!("Bad input to invoke")
  }
}
