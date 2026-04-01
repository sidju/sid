mod control_flow;
mod ffi;
mod scope;
mod stack;
mod type_ops;

use std::collections::HashMap;

use crate::invoke::ExeState;
use crate::type_system::SidType;
use crate::DataValue;

pub struct BuiltinEntry {
    pub name: &'static str,
    pub args: Vec<SidType>,
    pub ret: Vec<SidType>,
    pub exec: fn(&mut ExeState, Vec<DataValue>) -> Vec<DataValue>,
}

pub fn default_scope() -> HashMap<String, DataValue> {
    let types = DataValue::Map(vec![
        (
            DataValue::Label("bool".to_owned()),
            DataValue::Type(SidType::Bool),
        ),
        (
            DataValue::Label("int".to_owned()),
            DataValue::Type(SidType::Int),
        ),
        (
            DataValue::Label("float".to_owned()),
            DataValue::Type(SidType::Float),
        ),
        (
            DataValue::Label("char".to_owned()),
            DataValue::Type(SidType::Char),
        ),
        (
            DataValue::Label("str".to_owned()),
            DataValue::Type(SidType::Str),
        ),
        (
            DataValue::Label("label".to_owned()),
            DataValue::Type(SidType::Label),
        ),
        (
            DataValue::Label("any".to_owned()),
            DataValue::Type(SidType::Any),
        ),
        (
            DataValue::Label("value".to_owned()),
            DataValue::Type(SidType::Value),
        ),
        (
            DataValue::Label("null".to_owned()),
            DataValue::Pointer {
                addr: 0,
                pointee_ty: SidType::Any,
            },
        ),
    ]);
    let mut m = HashMap::new();
    m.insert("types".to_owned(), types);
    m
}

fn register_shared(m: &mut HashMap<&'static str, BuiltinEntry>) {
    m.insert("get", scope::get());
    m.insert("get_local", scope::get_local());
    m.insert("get_global", scope::get_global());
    m.insert("load_scope", scope::load_scope());
    m.insert("local", scope::local());
    m.insert("load_local", scope::load_local());
    m.insert("clone", stack::clone());
    m.insert("drop", stack::drop());
    m.insert("eq", stack::eq());
    m.insert("assert", stack::assert_builtin());
    m.insert("not", stack::not());
    m.insert("debug_stack", stack::debug_stack());
    m.insert("c_load_header", ffi::c_load_header());
    m.insert("ptr_cast", ffi::ptr_cast());
    m.insert("fn", type_ops::fn_type());
    m.insert("ptr", type_ops::ptr_type());
    m.insert("list", type_ops::list_type());
    m.insert("require", type_ops::require_type());
    m.insert("exclude", type_ops::exclude_type());
    m.insert("typed_args", type_ops::typed_args());
    m.insert("typed_rets", type_ops::typed_rets());
    m.insert("untyped_args", type_ops::untyped_args());
    m.insert("untyped_rets", type_ops::untyped_rets());
}

pub fn get_interpret_builtins() -> HashMap<&'static str, BuiltinEntry> {
    let mut m = HashMap::new();
    register_shared(&mut m);
    m.insert("c_link_lib", ffi::c_link_lib());
    m.insert("ptr_read_cstr", ffi::ptr_read_cstr());
    m.insert("while_do", control_flow::while_do());
    m.insert("do_while", control_flow::do_while());
    m.insert("match", control_flow::match_builtin());
    m
}

pub fn get_comptime_builtins() -> HashMap<&'static str, BuiltinEntry> {
    let mut m = HashMap::new();
    register_shared(&mut m);
    m
}
