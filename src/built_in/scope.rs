use crate::built_in::BuiltinEntry;
use crate::type_system::SidType;
use crate::{get_from_scope, DataValue};

fn pop_arg(args: &mut Vec<DataValue>, name: &str) -> DataValue {
    args.pop()
        .unwrap_or_else(|| panic!("{}: expected an argument", name))
}

fn pop_label(args: &mut Vec<DataValue>, name: &str) -> String {
    match pop_arg(args, name) {
        DataValue::Label(l) => l,
        other => panic!("{}: expected a label, got {:?}", name, other),
    }
}

pub fn get() -> BuiltinEntry {
    BuiltinEntry {
        name: "get",
        args: vec![SidType::Label],
        ret: vec![SidType::Any],
        exec: |state, mut args| {
            let label = pop_label(&mut args, "get");
            let builtin_names: std::collections::HashSet<&'static str> =
                state.builtins.keys().copied().collect();
            vec![get_from_scope(
                &label,
                Some(&state.local_scope),
                Some(state.global_state.scope),
                Some(&builtin_names),
            )
            .unwrap_or_else(|_| panic!("get: '{}' not found", label))]
        },
    }
}

pub fn get_local() -> BuiltinEntry {
    BuiltinEntry {
        name: "get_local",
        args: vec![SidType::Label],
        ret: vec![SidType::Any],
        exec: |state, mut args| {
            let label = pop_label(&mut args, "get_local");
            let builtin_names: std::collections::HashSet<&'static str> =
                state.builtins.keys().copied().collect();
            vec![
                get_from_scope(&label, Some(&state.local_scope), None, Some(&builtin_names))
                    .unwrap_or_else(|_| panic!("get_local: '{}' not found in local scope", label)),
            ]
        },
    }
}

pub fn get_global() -> BuiltinEntry {
    BuiltinEntry {
        name: "get_global",
        args: vec![SidType::Label],
        ret: vec![SidType::Any],
        exec: |state, mut args| {
            let label = pop_label(&mut args, "get_global");
            let builtin_names: std::collections::HashSet<&'static str> =
                state.builtins.keys().copied().collect();
            vec![get_from_scope(
                &label,
                None,
                Some(state.global_state.scope),
                Some(&builtin_names),
            )
            .unwrap_or_else(|_| panic!("get_global: '{}' not found in global scope", label))]
        },
    }
}

pub fn local() -> BuiltinEntry {
    BuiltinEntry {
        name: "local",
        args: vec![SidType::Any, SidType::Label],
        ret: vec![],
        exec: |state, mut args| {
            let value = pop_arg(&mut args, "local");
            let name = pop_label(&mut args, "local");
            state.local_scope.insert(name, value);
            vec![]
        },
    }
}

pub fn load_local() -> BuiltinEntry {
    BuiltinEntry {
        name: "load_local",
        args: vec![SidType::Any],
        ret: vec![],
        exec: |state, mut args| {
            let entries = match pop_arg(&mut args, "load_local") {
                DataValue::Map(e) => e,
                other => panic!("load_local expects a label-keyed Map, got {:?}", other),
            };
            for (key, value) in entries {
                match key {
                    DataValue::Label(name) => {
                        state.local_scope.insert(name, value);
                    }
                    other => panic!("load_local: key must be a Label, got {:?}", other),
                }
            }
            vec![]
        },
    }
}

pub fn load_scope() -> BuiltinEntry {
    BuiltinEntry {
        name: "load_scope",
        args: vec![SidType::Any],
        ret: vec![],
        exec: |state, mut args| {
            let entries = match pop_arg(&mut args, "load_scope") {
                DataValue::Map(e) => e,
                other => panic!("load_scope expects a label-keyed Map, got {:?}", other),
            };
            for (key, value) in entries {
                match key {
                    DataValue::Label(name) => {
                        state.global_state.scope.insert(name, value);
                    }
                    other => panic!("load_scope: key must be a Label, got {:?}", other),
                }
            }
            vec![]
        },
    }
}
