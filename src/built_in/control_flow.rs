use crate::built_in::BuiltinEntry;
use crate::type_system::SidType;
use crate::{DataValue, ProgramValue};

fn pop_arg(args: &mut Vec<DataValue>, name: &str) -> DataValue {
    args.pop()
        .unwrap_or_else(|| panic!("{}: expected an argument", name))
}

pub fn while_do() -> BuiltinEntry {
    BuiltinEntry {
        name: "while_do",
        args: vec![SidType::Any, SidType::Any],
        ret: vec![],
        exec: |state, mut args| {
            let body = pop_arg(&mut args, "while_do");
            let cond = pop_arg(&mut args, "while_do");
            match &body {
                DataValue::Substack { .. } => {}
                other => panic!("while_do: body must be a Substack, got {:?}", other),
            }
            match &cond {
                DataValue::Substack { .. } => {}
                other => panic!("while_do: condition must be a Substack, got {:?}", other),
            }
            state.program_stack.push(ProgramValue::CondLoopStart {
                cond: cond.clone(),
                body,
            });
            state.program_stack.push(ProgramValue::Invoke);
            state.program_stack.push(ProgramValue::Data(cond));
            vec![]
        },
    }
}

pub fn do_while() -> BuiltinEntry {
    BuiltinEntry {
        name: "do_while",
        args: vec![SidType::Any, SidType::Any],
        ret: vec![],
        exec: |state, mut args| {
            let cond = pop_arg(&mut args, "do_while");
            let body = pop_arg(&mut args, "do_while");
            match &cond {
                DataValue::Substack { .. } => {}
                other => panic!("do_while: condition must be a Substack, got {:?}", other),
            }
            match &body {
                DataValue::Substack { .. } => {}
                other => panic!("do_while: body must be a Substack, got {:?}", other),
            }
            let expected_len = state.data_stack.len();
            state.program_stack.push(ProgramValue::CondLoop {
                cond: cond.clone(),
                body: body.clone(),
                expected_len,
            });
            state.program_stack.push(ProgramValue::Invoke);
            state.program_stack.push(ProgramValue::Data(cond));
            state.program_stack.push(ProgramValue::Invoke);
            state.program_stack.push(ProgramValue::Data(body));
            vec![]
        },
    }
}

pub fn match_builtin() -> BuiltinEntry {
    BuiltinEntry {
        name: "match",
        args: vec![SidType::Any, SidType::Any],
        ret: vec![],
        exec: |state, mut args| {
            let cases = pop_arg(&mut args, "match");
            let value = pop_arg(&mut args, "match");
            let entries = match cases {
                DataValue::Map(e) => e,
                other => panic!("match: cases must be a Map, got {:?}", other),
            };
            for (pattern, action) in entries {
                if pattern.pattern_matches(&value) {
                    let body = match action {
                        DataValue::Substack { body, .. } | DataValue::Script { body, .. } => body,
                        other => panic!(
                            "match: action must be a Substack or Script, got {:?}",
                            other
                        ),
                    };
                    state.program_stack.extend(body.into_iter().rev());
                    return vec![];
                }
            }
            panic!("match: no case matched value {:?}", value);
        },
    }
}
