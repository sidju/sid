use std::collections::HashMap;

use sid::*;

fn mock_double(_state: &mut sid::ExeState, args: Vec<DataValue>) -> Vec<DataValue> {
    match args.into_iter().next() {
        Some(DataValue::Int(n)) => vec![DataValue::Int(n * 2)],
        Some(other) => panic!("MockDouble: expected Int, got {:?}", other),
        None => panic!("MockDouble: expected an argument"),
    }
}

fn mock_drop(_state: &mut sid::ExeState, args: Vec<DataValue>) -> Vec<DataValue> {
    if args.is_empty() {
        panic!("MockDrop: expected an argument");
    }
    vec![]
}

fn mock_const(_state: &mut sid::ExeState, _args: Vec<DataValue>) -> Vec<DataValue> {
    vec![DataValue::Int(42)]
}

pub struct ComptimePassFixture {
    pub input: Vec<TemplateValue>,
    pub expected_output: Vec<TemplateValue>,
}
impl ComptimePassFixture {
    pub fn test(&self, builtins: &HashMap<&'static str, sid::BuiltinEntry>) {
        let result = comptime_pass(self.input.clone(), builtins, &mut HashMap::new())
            .expect("comptime_pass failed unexpectedly");
        assert_eq!(
            result, self.expected_output,
            "comptime_pass output didn't match expectations"
        );
    }
}

pub struct ComptimeErrorFixture {
    pub input: Vec<TemplateValue>,
}
impl ComptimeErrorFixture {
    pub fn test(&self, builtins: &HashMap<&'static str, sid::BuiltinEntry>) {
        assert!(
            comptime_pass(self.input.clone(), builtins, &mut HashMap::new()).is_err(),
            "expected comptime_pass to return Err but it succeeded"
        );
    }
}

fn no_builtins() -> HashMap<&'static str, sid::BuiltinEntry> {
    HashMap::new()
}

fn comptime_invoke() -> TemplateValue {
    ProgramValue::ComptimeInvoke.into()
}

fn label(s: &str) -> TemplateValue {
    DataValue::Label(s.to_owned()).into()
}

#[test]
fn passthrough_plain_values() {
    ComptimePassFixture {
        input: vec![
            DataValue::Int(1).into(),
            DataValue::Str(std::ffi::CString::new("hi").unwrap()).into(),
        ],
        expected_output: vec![
            DataValue::Int(1).into(),
            DataValue::Str(std::ffi::CString::new("hi").unwrap()).into(),
        ],
    }
    .test(&no_builtins());
}

#[test]
fn passthrough_runtime_invoke() {
    ComptimePassFixture {
        input: vec![
            DataValue::Int(5).into(),
            label("something"),
            ProgramValue::Invoke.into(),
        ],
        expected_output: vec![
            DataValue::Int(5).into(),
            label("something"),
            ProgramValue::Invoke.into(),
        ],
    }
    .test(&no_builtins());
}

#[test]
fn passthrough_parent_refs() {
    ComptimePassFixture {
        input: vec![
            TemplateValue::ParentStackMove(1),
            TemplateValue::ParentLabel("foo".to_owned()),
        ],
        expected_output: vec![
            TemplateValue::ParentStackMove(1),
            TemplateValue::ParentLabel("foo".to_owned()),
        ],
    }
    .test(&no_builtins());
}

#[test]
fn comptime_invoke_one_arg_one_return() {
    let mut builtins: HashMap<&'static str, sid::BuiltinEntry> = HashMap::new();
    builtins.insert(
        "double",
        sid::BuiltinEntry {
            name: "double",
            args: vec![SidType::Int],
            ret: vec![SidType::Int],
            exec: mock_double,
        },
    );

    ComptimePassFixture {
        input: vec![DataValue::Int(5).into(), label("double"), comptime_invoke()],
        expected_output: vec![DataValue::Int(10).into()],
    }
    .test(&builtins);
}

#[test]
fn comptime_invoke_one_arg_zero_return() {
    let mut builtins: HashMap<&'static str, sid::BuiltinEntry> = HashMap::new();
    builtins.insert(
        "drop",
        sid::BuiltinEntry {
            name: "drop",
            args: vec![SidType::Any],
            ret: vec![],
            exec: mock_drop,
        },
    );

    ComptimePassFixture {
        input: vec![DataValue::Int(5).into(), label("drop"), comptime_invoke()],
        expected_output: vec![],
    }
    .test(&builtins);
}

#[test]
fn comptime_invoke_zero_arg_one_return() {
    let mut builtins: HashMap<&'static str, sid::BuiltinEntry> = HashMap::new();
    builtins.insert(
        "const",
        sid::BuiltinEntry {
            name: "const",
            args: vec![],
            ret: vec![SidType::Int],
            exec: mock_const,
        },
    );

    ComptimePassFixture {
        input: vec![label("const"), comptime_invoke()],
        expected_output: vec![DataValue::Int(42).into()],
    }
    .test(&builtins);
}

#[test]
fn comptime_invoke_leaves_surrounding_stack_intact() {
    let mut builtins: HashMap<&'static str, sid::BuiltinEntry> = HashMap::new();
    builtins.insert(
        "double",
        sid::BuiltinEntry {
            name: "double",
            args: vec![SidType::Int],
            ret: vec![SidType::Int],
            exec: mock_double,
        },
    );

    ComptimePassFixture {
        input: vec![
            DataValue::Str(std::ffi::CString::new("before").unwrap()).into(),
            DataValue::Int(3).into(),
            label("double"),
            comptime_invoke(),
            DataValue::Str(std::ffi::CString::new("after").unwrap()).into(),
        ],
        expected_output: vec![
            DataValue::Str(std::ffi::CString::new("before").unwrap()).into(),
            DataValue::Int(6).into(),
            DataValue::Str(std::ffi::CString::new("after").unwrap()).into(),
        ],
    }
    .test(&builtins);
}

#[test]
fn recurses_into_substack_body() {
    let mut builtins: HashMap<&'static str, sid::BuiltinEntry> = HashMap::new();
    builtins.insert(
        "double",
        sid::BuiltinEntry {
            name: "double",
            args: vec![SidType::Int],
            ret: vec![SidType::Int],
            exec: mock_double,
        },
    );

    let input_body = Template::substack((
        vec![DataValue::Int(5).into(), label("double"), comptime_invoke()],
        0,
    ));
    let expected_body = Template::substack((vec![DataValue::Int(10).into()], 0));
    ComptimePassFixture {
        input: vec![TemplateValue::Literal(ProgramValue::Template(input_body))],
        expected_output: vec![TemplateValue::Literal(ProgramValue::Template(
            expected_body,
        ))],
    }
    .test(&builtins);
}

#[test]
fn recurses_into_list_body() {
    let mut builtins: HashMap<&'static str, sid::BuiltinEntry> = HashMap::new();
    builtins.insert(
        "double",
        sid::BuiltinEntry {
            name: "double",
            args: vec![SidType::Int],
            ret: vec![SidType::Int],
            exec: mock_double,
        },
    );

    let input_body = Template::list((
        vec![DataValue::Int(3).into(), label("double"), comptime_invoke()],
        0,
    ));
    let expected_body = Template::list((vec![DataValue::Int(6).into()], 0));
    ComptimePassFixture {
        input: vec![TemplateValue::Literal(ProgramValue::Template(input_body))],
        expected_output: vec![TemplateValue::Literal(ProgramValue::Template(
            expected_body,
        ))],
    }
    .test(&builtins);
}

#[test]
fn recurses_into_script_body() {
    let mut builtins: HashMap<&'static str, sid::BuiltinEntry> = HashMap::new();
    builtins.insert(
        "double",
        sid::BuiltinEntry {
            name: "double",
            args: vec![SidType::Int],
            ret: vec![SidType::Int],
            exec: mock_double,
        },
    );

    let input_body = Template::script((
        vec![DataValue::Int(7).into(), label("double"), comptime_invoke()],
        0,
    ));
    let expected_body = Template::script((vec![DataValue::Int(14).into()], 0));
    ComptimePassFixture {
        input: vec![TemplateValue::Literal(ProgramValue::Template(input_body))],
        expected_output: vec![TemplateValue::Literal(ProgramValue::Template(
            expected_body,
        ))],
    }
    .test(&builtins);
}

#[test]
fn error_comptime_invoke_unknown_function() {
    ComptimeErrorFixture {
        input: vec![
            DataValue::Int(5).into(),
            label("nonexistent"),
            comptime_invoke(),
        ],
    }
    .test(&no_builtins());
}

#[test]
fn error_comptime_invoke_unrendered_template_as_arg() {
    let mut builtins: HashMap<&'static str, sid::BuiltinEntry> = HashMap::new();
    builtins.insert(
        "double",
        sid::BuiltinEntry {
            name: "double",
            args: vec![SidType::Int],
            ret: vec![SidType::Int],
            exec: mock_double,
        },
    );

    let unrendered = Template::substack((vec![], 0));
    ComptimeErrorFixture {
        input: vec![
            TemplateValue::Literal(ProgramValue::Template(unrendered)),
            label("double"),
            comptime_invoke(),
        ],
    }
    .test(&builtins);
}

#[test]
fn error_comptime_invoke_parent_ref_as_arg() {
    let mut builtins: HashMap<&'static str, sid::BuiltinEntry> = HashMap::new();
    builtins.insert(
        "double",
        sid::BuiltinEntry {
            name: "double",
            args: vec![SidType::Int],
            ret: vec![SidType::Int],
            exec: mock_double,
        },
    );

    ComptimeErrorFixture {
        input: vec![
            TemplateValue::ParentStackMove(1),
            label("double"),
            comptime_invoke(),
        ],
    }
    .test(&builtins);
}
