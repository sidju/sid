use sid::*;

use std::collections::HashMap;

pub fn render_test_fixture(
    template: Template,
    mut parent_stack: Vec<TemplateValue>,
    parent_scope: HashMap<String, DataValue>,
    mut global_scope: HashMap<String, DataValue>,
    expected_parent_stack: Vec<TemplateValue>,
    expected_rendered: DataValue,
) {
    let builtins = get_interpret_builtins();
    let rendered = {
        let mut gs = GlobalState::new(&mut global_scope);
        render_template(
            template,
            &mut parent_stack,
            &parent_scope,
            &mut gs,
            &builtins,
        )
    };
    // Verify remaining parent stack
    assert_eq!(
        parent_stack, expected_parent_stack,
        "Parent stack after render wasn't as expected"
    );
    // Verify rendered value
    assert_eq!(
        rendered, expected_rendered,
        "Rendered value wasn't as expected."
    );
}

#[test]
fn render_empty_substack() {
    render_test_fixture(
        // Template
        Template::substack((vec![], 0)),
        // Parent stack
        vec![],
        // Parent and global scope, respectively
        HashMap::new(),
        HashMap::new(),
        // Expected parent stack
        vec![],
        // Expected rendered value
        DataValue::Substack {
            body: vec![],
            args: None,
            ret: None,
        },
    )
}

#[test]
fn render_substack() {
    let mut global = HashMap::new();
    global.insert("one".to_string(), DataValue::Int(1));
    render_test_fixture(
        // Template
        Template::substack((
            vec![
                TemplateValue::ParentStackMove(1),
                TemplateValue::ParentLabel("one".to_string()),
                DataValue::Label("add".to_string()).into(),
                ProgramValue::Invoke.into(),
            ],
            1,
        )),
        // Parent stack
        vec![DataValue::Bool(true).into(), DataValue::Int(2).into()],
        // Parent and global scope, respectively
        HashMap::new(),
        global,
        // Expected parent stack
        vec![DataValue::Bool(true).into()],
        // Expected rendered value
        DataValue::Substack {
            body: vec![
                DataValue::Int(2).into(),
                DataValue::Int(1).into(),
                DataValue::Label("add".to_string()).into(),
                ProgramValue::Invoke.into(),
            ],
            args: None,
            ret: None,
        },
    )
}

#[test]
fn render_list() {
    render_test_fixture(
        Template::list((
            vec![
                DataValue::Int(1).into(),
                DataValue::Int(2).into(),
                DataValue::Int(3).into(),
            ],
            0,
        )),
        vec![],
        HashMap::new(),
        HashMap::new(),
        vec![],
        DataValue::List(vec![
            DataValue::Int(1),
            DataValue::Int(2),
            DataValue::Int(3),
        ]),
    )
}

#[test]
fn render_set() {
    render_test_fixture(
        Template::set((
            vec![
                DataValue::Str(std::ffi::CString::new("a").unwrap()).into(),
                DataValue::Str(std::ffi::CString::new("b").unwrap()).into(),
            ],
            0,
        )),
        vec![],
        HashMap::new(),
        HashMap::new(),
        vec![],
        DataValue::Set(vec![
            DataValue::Str(std::ffi::CString::new("a").unwrap()),
            DataValue::Str(std::ffi::CString::new("b").unwrap()),
        ]),
    )
}

#[test]
fn render_map() {
    render_test_fixture(
        Template::map(
            vec![
                (
                    vec![DataValue::Label("x".to_owned()).into()],
                    vec![DataValue::Int(1).into()],
                ),
                (
                    vec![DataValue::Label("y".to_owned()).into()],
                    vec![DataValue::Int(2).into()],
                ),
            ],
            0,
        ),
        vec![],
        HashMap::new(),
        HashMap::new(),
        vec![],
        DataValue::Map(vec![
            (DataValue::Label("x".to_owned()), DataValue::Int(1)),
            (DataValue::Label("y".to_owned()), DataValue::Int(2)),
        ]),
    )
}

#[test]
fn render_script() {
    render_test_fixture(
        Template::script((vec![DataValue::Int(42).into()], 0)),
        vec![],
        HashMap::new(),
        HashMap::new(),
        vec![],
        DataValue::Script {
            body: vec![ProgramValue::Data(DataValue::Int(42))],
            args: None,
            ret: None,
        },
    )
}

// ── $N duplication (clone equivalent) ────────────────────────────────────────

/// A substack that references `$1` twice should render a body containing
/// two copies of the consumed value — the template-level equivalent of `clone!`.
#[test]
fn render_substack_dollar_one_twice() {
    render_test_fixture(
        // Template: ($1 $1) — consumes 1 entry, references it twice
        Template::substack((
            vec![
                TemplateValue::ParentStackMove(1),
                TemplateValue::ParentStackMove(1),
            ],
            1,
        )),
        // Parent stack: [Int(7)]
        vec![DataValue::Int(7).into()],
        HashMap::new(),
        HashMap::new(),
        // Parent stack is drained
        vec![],
        // Rendered substack body has two copies of 7
        DataValue::Substack {
            body: vec![DataValue::Int(7).into(), DataValue::Int(7).into()],
            args: None,
            ret: None,
        },
    )
}

// ── implicit drop of unused consumed slots ───────────────────────────────────

/// A substack that consumes 2 entries but only references `$2` should render
/// a body containing only the second value — the first is implicitly dropped.
/// This is the template-level equivalent of `drop!`.
#[test]
fn render_substack_dollar_two_drops_first() {
    render_test_fixture(
        // Template: ($2) — consumes 2 entries, only references $2
        Template::substack((vec![TemplateValue::ParentStackMove(2)], 2)),
        // Parent stack: [Int(1), Int(2)]  (2 is on top)
        vec![DataValue::Int(1).into(), DataValue::Int(2).into()],
        HashMap::new(),
        HashMap::new(),
        // Parent stack is drained
        vec![],
        // Rendered substack body has only the second value (2); the first is dropped
        DataValue::Substack {
            body: vec![DataValue::Int(2).into()],
            args: None,
            ret: None,
        },
    )
}

/// Consuming 3 entries and only referencing `$3` drops the first two.
#[test]
fn render_substack_dollar_three_drops_first_two() {
    render_test_fixture(
        Template::substack((vec![TemplateValue::ParentStackMove(3)], 3)),
        vec![
            DataValue::Int(1).into(),
            DataValue::Int(2).into(),
            DataValue::Int(3).into(),
        ],
        HashMap::new(),
        HashMap::new(),
        vec![],
        DataValue::Substack {
            body: vec![DataValue::Int(3).into()],
            args: None,
            ret: None,
        },
    )
}
