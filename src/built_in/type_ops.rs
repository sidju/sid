use crate::built_in::BuiltinEntry;
use crate::type_system::SidType;
use crate::DataValue;

fn pop_arg(args: &mut Vec<DataValue>, name: &str) -> DataValue {
    args.pop()
        .unwrap_or_else(|| panic!("{}: expected an argument", name))
}

pub fn fn_type() -> BuiltinEntry {
    BuiltinEntry {
        name: "fn",
        args: vec![],
        ret: vec![SidType::Any],
        exec: |_state, _args| {
            vec![DataValue::Type(SidType::Fn {
                args: None,
                ret: None,
            })]
        },
    }
}

pub fn ptr_type() -> BuiltinEntry {
    BuiltinEntry {
        name: "ptr",
        args: vec![SidType::Any],
        ret: vec![SidType::Any],
        exec: |_state, mut args| {
            let raw = pop_arg(&mut args, "ptr");
            let inner = match raw {
                DataValue::Type(t) => t,
                other => panic!("ptr: expected a type, got {:?}", other),
            };
            vec![DataValue::Type(SidType::Pointer(Box::new(inner)))]
        },
    }
}

pub fn list_type() -> BuiltinEntry {
    BuiltinEntry {
        name: "list",
        args: vec![SidType::Any],
        ret: vec![SidType::Any],
        exec: |_state, mut args| {
            let raw = pop_arg(&mut args, "list");
            let inner = match raw {
                DataValue::Type(t) => t,
                other => panic!("list: expected a type, got {:?}", other),
            };
            vec![DataValue::Type(SidType::List(Box::new(inner)))]
        },
    }
}

pub fn require_type() -> BuiltinEntry {
    BuiltinEntry {
        name: "require",
        args: vec![SidType::Any, SidType::Any],
        ret: vec![SidType::Any],
        exec: |_state, mut args| {
            let constraint = pop_arg(&mut args, "require");
            let base = pop_arg(&mut args, "require");
            let resolve_type = |raw: DataValue| -> SidType {
                match raw {
                    DataValue::Type(t) => t,
                    other => SidType::Literal(Box::new(other)),
                }
            };
            vec![DataValue::Type(SidType::Require {
                base: Box::new(resolve_type(base)),
                constraint: Box::new(resolve_type(constraint)),
            })]
        },
    }
}

pub fn exclude_type() -> BuiltinEntry {
    BuiltinEntry {
        name: "exclude",
        args: vec![SidType::Any, SidType::Any],
        ret: vec![SidType::Any],
        exec: |_state, mut args| {
            let forbidden = pop_arg(&mut args, "exclude");
            let base = pop_arg(&mut args, "exclude");
            let resolve_type = |raw: DataValue| -> SidType {
                match raw {
                    DataValue::Type(t) => t,
                    other => SidType::Literal(Box::new(other)),
                }
            };
            vec![DataValue::Type(SidType::Exclude {
                base: Box::new(resolve_type(base)),
                forbidden: Box::new(resolve_type(forbidden)),
            })]
        },
    }
}

fn list_to_type_vec(list: DataValue, ctx: &str) -> Vec<SidType> {
    match list {
        DataValue::List(items) => items
            .into_iter()
            .map(|v| match v {
                DataValue::Type(t) => t,
                other => panic!("{}: expected a list of types, got {:?}", ctx, other),
            })
            .collect(),
        other => panic!("{}: expected a List of types, got {:?}", ctx, other),
    }
}

pub fn typed_args() -> BuiltinEntry {
    BuiltinEntry {
        name: "typed_args",
        args: vec![SidType::Any, SidType::Any],
        ret: vec![SidType::Any],
        exec: |_state, mut args| {
            let target_val = pop_arg(&mut args, "typed_args");
            let types_val = pop_arg(&mut args, "typed_args");
            let label_map_ty = SidType::Map {
                key: Box::new(SidType::Label),
                value: Box::new(SidType::Any),
            };
            if !label_map_ty.matches(&types_val) {
                panic!(
                    "typed_args: expected a label-keyed Map, got {:?}",
                    types_val
                );
            }
            let entries = match types_val {
                DataValue::Map(e) => e,
                _ => unreachable!(),
            };
            let named_args: Vec<(String, SidType)> = entries
                .into_iter()
                .rev()
                .map(|(k, v)| {
                    let name = match k {
                        DataValue::Label(s) => s,
                        _ => unreachable!(),
                    };
                    let ty = match v {
                        DataValue::Type(t) => t,
                        other => panic!(
                            "typed_args: field '{}' value must be a type, got {:?}",
                            name, other
                        ),
                    };
                    (name, ty)
                })
                .collect();
            let type_only: Vec<SidType> = named_args.iter().map(|(_, t)| t.clone()).collect();
            vec![match target_val {
                DataValue::Substack { body, ret, .. } => DataValue::Substack {
                    body,
                    args: Some(named_args),
                    ret,
                },
                DataValue::Script { body, ret, .. } => DataValue::Script {
                    body,
                    args: Some(named_args),
                    ret,
                },
                DataValue::Type(SidType::Fn { ret, .. }) => DataValue::Type(SidType::Fn {
                    args: Some(type_only),
                    ret,
                }),
                other => panic!(
                    "typed_args: expected Substack, Script, or Fn type, got {:?}",
                    other
                ),
            }]
        },
    }
}

pub fn typed_rets() -> BuiltinEntry {
    BuiltinEntry {
        name: "typed_rets",
        args: vec![SidType::Any, SidType::Any],
        ret: vec![SidType::Any],
        exec: |_state, mut args| {
            let target_val = pop_arg(&mut args, "typed_rets");
            let types_val = pop_arg(&mut args, "typed_rets");
            let types: Vec<SidType> = list_to_type_vec(types_val, "typed_rets")
                .into_iter()
                .rev()
                .collect();
            vec![match target_val {
                DataValue::Substack { body, args, .. } => DataValue::Substack {
                    body,
                    args,
                    ret: Some(types),
                },
                DataValue::Script { body, args, .. } => DataValue::Script {
                    body,
                    args,
                    ret: Some(types),
                },
                DataValue::Type(SidType::Fn { args, .. }) => DataValue::Type(SidType::Fn {
                    args,
                    ret: Some(types),
                }),
                other => panic!(
                    "typed_rets: expected Substack, Script, or Fn type, got {:?}",
                    other
                ),
            }]
        },
    }
}

pub fn untyped_args() -> BuiltinEntry {
    BuiltinEntry {
        name: "untyped_args",
        args: vec![SidType::Any],
        ret: vec![SidType::Any],
        exec: |_state, mut args| {
            let target_val = pop_arg(&mut args, "untyped_args");
            vec![match target_val {
                DataValue::Substack { body, ret, .. } => DataValue::Substack {
                    body,
                    args: None,
                    ret,
                },
                DataValue::Script { body, ret, .. } => DataValue::Script {
                    body,
                    args: None,
                    ret,
                },
                DataValue::Type(SidType::Fn { ret, .. }) => {
                    DataValue::Type(SidType::Fn { args: None, ret })
                }
                other => panic!(
                    "untyped_args: expected Substack, Script, or Fn type, got {:?}",
                    other
                ),
            }]
        },
    }
}

pub fn untyped_rets() -> BuiltinEntry {
    BuiltinEntry {
        name: "untyped_rets",
        args: vec![SidType::Any],
        ret: vec![SidType::Any],
        exec: |_state, mut args| {
            let target_val = pop_arg(&mut args, "untyped_rets");
            vec![match target_val {
                DataValue::Substack { body, args, .. } => DataValue::Substack {
                    body,
                    args,
                    ret: None,
                },
                DataValue::Script { body, args, .. } => DataValue::Script {
                    body,
                    args,
                    ret: None,
                },
                DataValue::Type(SidType::Fn { args, .. }) => {
                    DataValue::Type(SidType::Fn { args, ret: None })
                }
                other => panic!(
                    "untyped_rets: expected Substack, Script, or Fn type, got {:?}",
                    other
                ),
            }]
        },
    }
}
