use super::*;

#[derive(Debug, PartialEq)]
pub struct MockCall {
  pub func: SideEffectFunction,
  pub args: Vec<Value>,
}

pub struct MockSideEffector {
  pub calls: Vec<MockCall>,
}
impl SideEffector for MockSideEffector {
  fn invoke(&mut self,
    function: SideEffectFunction,
    stack: &mut Vec<Value>,
  ) {
    use SideEffectFunction as SEF;
    match function {
      SEF::Print => { self.calls.push(MockCall{
        func: function,
        args: vec![stack.pop().unwrap()],
      })},
    }
  }
}
