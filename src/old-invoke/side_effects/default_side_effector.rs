use super::*;

pub struct DefaultSideEffector {}
impl SideEffector for DefaultSideEffector {
  fn invoke(&mut self,
    function: SideEffectFunction,
    stack: &mut Vec<Value>,
  ) {
    use SideEffectFunction as SEF;
    match function {
      SEF::Print => self.print(stack),
    }
  }
}

impl DefaultSideEffector {
  fn print(&mut self,
    stack: &mut Vec<Value>,
  ) {
    let message = match stack.pop().map(realize_value) {
      Some(RealValue::Str(x)) => x,
      _ => panic!("Bad input to print function"),
    };
    print!("{}", message);
  }
}
