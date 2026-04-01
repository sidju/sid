use crate::built_in::BuiltinEntry;
use crate::type_system::SidType;
use crate::DataValue;

fn pop_arg(args: &mut Vec<DataValue>, name: &str) -> DataValue {
    args.pop()
        .unwrap_or_else(|| panic!("{}: expected an argument", name))
}

pub fn clone() -> BuiltinEntry {
    BuiltinEntry {
        name: "clone",
        args: vec![SidType::Any],
        ret: vec![SidType::Any, SidType::Any],
        exec: |_state, mut args| {
            let v = pop_arg(&mut args, "clone");
            vec![v.clone(), v]
        },
    }
}

pub fn drop() -> BuiltinEntry {
    BuiltinEntry {
        name: "drop",
        args: vec![SidType::Any],
        ret: vec![],
        exec: |_state, mut args| {
            pop_arg(&mut args, "drop");
            vec![]
        },
    }
}

pub fn eq() -> BuiltinEntry {
    BuiltinEntry {
        name: "eq",
        args: vec![SidType::Any, SidType::Any],
        ret: vec![SidType::Bool],
        exec: |_state, mut args| {
            let b = pop_arg(&mut args, "eq");
            let a = pop_arg(&mut args, "eq");
            vec![DataValue::Bool(a == b)]
        },
    }
}

pub fn assert_builtin() -> BuiltinEntry {
    BuiltinEntry {
        name: "assert",
        args: vec![SidType::Bool],
        ret: vec![],
        exec: |_state, mut args| match pop_arg(&mut args, "assert") {
            DataValue::Bool(true) => vec![],
            DataValue::Bool(false) => panic!("assertion failed"),
            other => panic!("assert expects Bool, got {:?}", other),
        },
    }
}

pub fn not() -> BuiltinEntry {
    BuiltinEntry {
        name: "not",
        args: vec![SidType::Bool],
        ret: vec![SidType::Bool],
        exec: |_state, mut args| match pop_arg(&mut args, "not") {
            DataValue::Bool(b) => vec![DataValue::Bool(!b)],
            other => panic!("not expects Bool, got {:?}", other),
        },
    }
}

pub fn debug_stack() -> BuiltinEntry {
    BuiltinEntry {
        name: "debug_stack",
        args: vec![SidType::Int],
        ret: vec![],
        exec: |_state, mut args| {
            let n = match pop_arg(&mut args, "debug_stack") {
                DataValue::Int(n) if n >= 0 => n as usize,
                DataValue::Int(n) => panic!("debug_stack: count must be non-negative, got {}", n),
                other => panic!("debug_stack expects Int, got {:?}", other),
            };
            eprintln!("=== debug_stack (top {}) ===", n);
            vec![]
        },
    }
}
