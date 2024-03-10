use super::*;

pub struct DefaultSideEffector {}
impl SideEffector for MockSideEffector {
  fn print(message: &str) {
    print!("{}", message);
  }
}
