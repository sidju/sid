// We create side-effects through a trait implementation
// (This allows mocking all side effects in one for testing)
pub mod type_system;
pub use type_system::SidType;
pub mod c_ffi;
/// Expose `open_library` for tests and integration code that needs to
/// pre-populate the library registry (e.g. after `c_link_lib` changes).
pub use c_ffi::open_library as c_ffi_open_library;
pub use c_ffi::{call_c_function, call_cfuncsig, parse_c_header, CFunc, CFuncSig, CType};
mod types;
pub use types::GlobalState;
pub use types::*;
mod to_syntax;
pub use to_syntax::*;
mod parse;
pub use parse::*;
mod render;
pub use render::*;
mod invoke;
pub use invoke::*;
mod built_in;
pub use built_in::*;
mod comptime;
pub use comptime::*;
#[cfg(feature = "llvm")]
pub mod llvm_backend;
//
//
//pub fn interpret_str(
//  script: &str,
//  side_effector: &mut dyn SideEffector,
//) -> Result<Vec<Value>, Box<dyn Error>> {
//  let mut scope = HashMap::new();
//  invoke(
//    parse_str(script),
//    side_effector,
//    scope,
//  )
//}
//
//pub fn render_template(
//  mut consumed_stack: Vec<Value>,
//  global_scope: &HashMap<String, RealValue>,
//  template: Template,
//) -> RealValue {
//  let mut consumed_stack: Vec<Option<RealValue>> = consumed_stack.drain(..).map(
//    |x| Some(realize_value(x))
//  ).collect();
//  use Template as T;
//  match template {
//    T::SubstackTemplate(source) => {
//      let mut rendered: Vec<ProgramValue> = Vec::new();
//      use TemplateValue as TV;
//      for entry in source { match entry {
//        TV::Literal(v) => { rendered.push(v); },
//        TV::ParentLabel(l) => { rendered.push(resolve_label(&l).into()); },
//        TV::ParentStackMove(i) => {
//          let value = consumed_stack[i].take().expect("Stack value taken twice in template");
//          rendered.push(value.into());
//        },
//      }}
//      RealValue::Substack(rendered)
//    },
//  }
//}
//
//// The public facing types should be:
////   TemplateValue, Value, (RealValue)
//// The public facing functions should be:
////   parse, render, invoke
//
