pub mod mock_side_effector;
pub mod default_side_effector;

pub trait SideEffector {
  fn print(message: &str);
}
