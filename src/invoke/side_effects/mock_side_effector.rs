use super::*;

pub struct MockCall {
  func: &'static str,
  args: Vec<Box<dyn std::any::Any>>,
}

pub struct MockSideEffector {
  pub calls: Vec<MockCall>,
}
impl SideEffector for MockSideEffector {
  fn print(self, message: &str) {
    self.calls.push(MockCall{
      func: "print",
      args: vec![Box::new(message.to_owned())],
    })
  }
}
