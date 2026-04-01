use std::collections::HashMap;
use std::path::PathBuf;

use sid::*;

// ── Helper to locate test fixtures ───────────────────────────────────────────

fn fixture_header() -> String {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("tests/fixtures/test.h");
    p.to_string_lossy().into_owned()
}

const TEST_LIB: &str = "libm.so.6";

// ── C header parsing tests ────────────────────────────────────────────────────

#[test]
fn parses_double_param_double_return() {
    let sigs = parse_c_header(&fixture_header(), TEST_LIB).expect("parse_c_header failed");
    let sqrt_sig = sigs
        .iter()
        .find(|s| s.name == "sqrt")
        .expect("sqrt not found");
    assert_eq!(sqrt_sig.ret, CType::Double);
    assert_eq!(sqrt_sig.params, vec![CType::Double]);
    assert_eq!(sqrt_sig.lib_name, TEST_LIB);
}

#[test]
fn parses_cstring_param_int_return() {
    let sigs = parse_c_header(&fixture_header(), TEST_LIB).expect("parse_c_header failed");
    let puts_sig = sigs
        .iter()
        .find(|s| s.name == "puts")
        .expect("puts not found");
    assert_eq!(puts_sig.ret, CType::Int);
    assert_eq!(puts_sig.params, vec![CType::CString]);
}

#[test]
fn includes_variadic_functions() {
    let sigs = parse_c_header(&fixture_header(), TEST_LIB).expect("parse_c_header failed");
    let printf = sigs
        .iter()
        .find(|s| s.name == "printf")
        .expect("printf (variadic) should be included");
    assert!(printf.variadic, "printf should be marked variadic");
    assert_eq!(
        printf.params.len(),
        1,
        "printf should have 1 fixed param (the format string)"
    );
}

#[test]
fn skips_struct_and_typedef_declarations() {
    let sigs = parse_c_header(&fixture_header(), TEST_LIB).expect("parse_c_header failed");
    for sig in &sigs {
        assert_ne!(sig.name, "Foo", "struct should be skipped");
        assert_ne!(sig.name, "MyInt", "typedef should be skipped");
    }
}

#[test]
fn error_on_nonexistent_header() {
    let result = parse_c_header("/nonexistent/path/header.h", TEST_LIB);
    assert!(result.is_err(), "should error on missing file");
}

// ── call_cfuncsig tests ───────────────────────────────────────────────────────

fn get_sqrt_sig() -> CFuncSig {
    parse_c_header(&fixture_header(), TEST_LIB)
        .expect("parse_c_header failed")
        .into_iter()
        .find(|s| s.name == "sqrt")
        .expect("sqrt not found in header")
}

#[test]
fn call_sqrt_returns_correct_value() {
    // c_link_lib must be called first; call_cfuncsig no longer lazy-loads.
    let mut libs = HashMap::new();
    libs.insert(
        TEST_LIB.to_owned(),
        sid::c_ffi_open_library(TEST_LIB).expect("open libm"),
    );

    let sig = get_sqrt_sig();
    let result =
        call_cfuncsig(&sig, Some(DataValue::Float(9.0)), &libs).expect("call_cfuncsig failed");
    match result {
        Some(DataValue::Float(v)) => {
            assert!(
                (v - 3.0).abs() < 1e-9,
                "sqrt(9.0) should be ~3.0, got {}",
                v
            );
        }
        other => panic!("expected DataValue::Float, got {:?}", other),
    }
}

#[test]
fn call_with_wrong_arg_count_errors() {
    let mut libs = HashMap::new();
    libs.insert(
        TEST_LIB.to_owned(),
        sid::c_ffi_open_library(TEST_LIB).expect("open libm"),
    );
    let sig = get_sqrt_sig();
    let result = call_cfuncsig(&sig, None, &libs);
    assert!(result.is_err(), "wrong arg count should return Err");
}

#[test]
fn error_on_unloaded_library() {
    // Library NOT pre-loaded — must be a fatal error.
    let sig = get_sqrt_sig();
    let result = call_cfuncsig(&sig, Some(DataValue::Float(1.0)), &HashMap::new());
    assert!(result.is_err(), "should error when library is not loaded");
}

// ── CFuncSig invoke via interpret ─────────────────────────────────────────────

#[test]
fn interpret_cfuncsig_in_global_scope() {
    let sqrt_sig = get_sqrt_sig();

    let mut global_scope = HashMap::new();
    let mut global_state = GlobalState::new(&mut global_scope);
    // Pre-load the library so call_cfuncsig doesn't error on missing lib.
    global_state.libraries.insert(
        TEST_LIB.to_owned(),
        sid::c_ffi_open_library(TEST_LIB).expect("open libm"),
    );
    global_state
        .scope
        .insert("sqrt".to_owned(), DataValue::CFuncSig(sqrt_sig));

    let program = vec![ProgramValue::Invoke];
    let data_stack = vec![
        DataValue::Float(16.0).into(),
        DataValue::Label("sqrt".to_owned()).into(),
    ];
    let builtins = get_interpret_builtins();

    interpret(program, data_stack, global_state, &builtins);
}

/// A label that resolves to a Float in scope is transparently passed as the
/// argument to a CFuncSig call — no type-mismatch error.
#[test]
fn interpret_cfuncsig_label_arg_resolved() {
    let sqrt_sig = get_sqrt_sig();

    let mut global_scope = HashMap::new();
    let mut global_state = GlobalState::new(&mut global_scope);
    global_state.libraries.insert(
        TEST_LIB.to_owned(),
        sid::c_ffi_open_library(TEST_LIB).expect("open libm"),
    );
    global_state
        .scope
        .insert("sqrt".to_owned(), DataValue::CFuncSig(sqrt_sig));
    global_state
        .scope
        .insert("my_val".to_owned(), DataValue::Float(9.0));

    let program = vec![ProgramValue::Invoke];
    let data_stack = vec![
        DataValue::Label("my_val".to_owned()).into(), // resolved → Float(9.0)
        DataValue::Label("sqrt".to_owned()).into(),
    ];
    let builtins = get_interpret_builtins();

    let mut data_stack_out = data_stack;
    let mut program_stack = program;
    let mut local_scope = HashMap::new();
    let mut scope_stack = Vec::new();
    while !program_stack.is_empty() {
        interpret_one(
            &mut data_stack_out,
            &mut program_stack,
            &mut local_scope,
            &mut scope_stack,
            &mut global_state,
            &builtins,
        );
    }
    match data_stack_out.as_slice() {
        [TemplateValue::Literal(ProgramValue::Data(DataValue::Float(v)))] => {
            assert!(
                (v - 3.0).abs() < 1e-9,
                "sqrt(9.0) should be ~3.0, got {}",
                v
            );
        }
        other => panic!("expected [Float(~3.0)], got {:?}", other),
    }
}

/// An undefined label passed as a CFuncSig argument panics with a clear
/// "undefined label" message rather than a cryptic type-mismatch error.
#[test]
#[should_panic(expected = "undefined label")]
fn interpret_cfuncsig_undefined_label_arg_panics() {
    let sqrt_sig = get_sqrt_sig();

    let mut global_scope = HashMap::new();
    let mut global_state = GlobalState::new(&mut global_scope);
    global_state.libraries.insert(
        TEST_LIB.to_owned(),
        sid::c_ffi_open_library(TEST_LIB).expect("open libm"),
    );
    global_state
        .scope
        .insert("sqrt".to_owned(), DataValue::CFuncSig(sqrt_sig));

    let program = vec![ProgramValue::Invoke];
    let data_stack = vec![
        DataValue::Label("no_such_val".to_owned()).into(), // undefined → panic
        DataValue::Label("sqrt".to_owned()).into(),
    ];
    let builtins = get_interpret_builtins();
    interpret(program, data_stack, global_state, &builtins);
}

/// Labels inside a variadic argument List are each resolved before the call.
/// Variadic functions use stack form: fixed params on stack, then a List of variadic args on top.
#[test]
fn interpret_cfuncsig_variadic_stack_form() {
    let sigs = parse_c_header(&fixture_header(), TEST_LIB).expect("parse_c_header failed");
    let printf_sig = sigs
        .into_iter()
        .find(|s| s.name == "printf")
        .expect("printf not found");

    let mut global_scope = HashMap::new();
    let mut global_state = GlobalState::new(&mut global_scope);
    global_state.libraries.insert(
        TEST_LIB.to_owned(),
        sid::c_ffi_open_library(TEST_LIB).expect("open libm"),
    );
    global_state
        .scope
        .insert("printf".to_owned(), DataValue::CFuncSig(printf_sig));

    let program = vec![ProgramValue::Invoke];
    let data_stack = vec![
        // format string (the 1 fixed param), then empty variadic list
        DataValue::Str(std::ffi::CString::new("").unwrap()).into(),
        DataValue::List(vec![]).into(), // variadic args (none)
        DataValue::Label("printf".to_owned()).into(),
    ];
    let builtins = get_interpret_builtins();
    interpret(program, data_stack, global_state, &builtins);
}

/// Multi-param non-variadic function: stack form pushes N items individually.
/// hypot(3.0, 4.0) should return 5.0.
#[test]
fn interpret_cfuncsig_multi_param_stack_form() {
    let sigs = parse_c_header(&fixture_header(), TEST_LIB).expect("parse_c_header failed");
    let hypot_sig = sigs
        .into_iter()
        .find(|s| s.name == "hypot")
        .expect("hypot not found");

    let mut global_scope = HashMap::new();
    let mut global_state = GlobalState::new(&mut global_scope);
    global_state.libraries.insert(
        TEST_LIB.to_owned(),
        sid::c_ffi_open_library(TEST_LIB).expect("open libm"),
    );
    global_state
        .scope
        .insert("hypot".to_owned(), DataValue::CFuncSig(hypot_sig));

    let program = vec![ProgramValue::Invoke];
    // Deepest = first declared param (x=3.0), top = last declared param (y=4.0).
    let data_stack = vec![
        DataValue::Float(3.0).into(),
        DataValue::Float(4.0).into(),
        DataValue::Label("hypot".to_owned()).into(),
    ];
    let builtins = get_interpret_builtins();

    let mut data_stack_out = data_stack;
    let mut program_stack = program;
    let mut local_scope = HashMap::new();
    let mut scope_stack = Vec::new();
    while !program_stack.is_empty() {
        interpret_one(
            &mut data_stack_out,
            &mut program_stack,
            &mut local_scope,
            &mut scope_stack,
            &mut global_state,
            &builtins,
        );
    }
    match data_stack_out.as_slice() {
        [TemplateValue::Literal(ProgramValue::Data(DataValue::Float(v)))] => {
            assert!(
                (v - 5.0).abs() < 1e-9,
                "hypot(3,4) should be ~5.0, got {}",
                v
            );
        }
        other => panic!("expected [Float(~5.0)], got {:?}", other),
    }
}

/// Multi-param non-variadic function: struct form passes a Map with param names as keys.
/// hypot({x: 3.0, y: 4.0}) should return 5.0.
#[test]
fn interpret_cfuncsig_multi_param_struct_form() {
    let sigs = parse_c_header(&fixture_header(), TEST_LIB).expect("parse_c_header failed");
    let hypot_sig = sigs
        .into_iter()
        .find(|s| s.name == "hypot")
        .expect("hypot not found");

    let mut global_scope = HashMap::new();
    let mut global_state = GlobalState::new(&mut global_scope);
    global_state.libraries.insert(
        TEST_LIB.to_owned(),
        sid::c_ffi_open_library(TEST_LIB).expect("open libm"),
    );
    global_state
        .scope
        .insert("hypot".to_owned(), DataValue::CFuncSig(hypot_sig));

    let program = vec![ProgramValue::Invoke];
    let data_stack = vec![
        DataValue::Map(vec![
            (DataValue::Label("x".to_owned()), DataValue::Float(3.0)),
            (DataValue::Label("y".to_owned()), DataValue::Float(4.0)),
        ])
        .into(),
        DataValue::Label("hypot".to_owned()).into(),
    ];
    let builtins = get_interpret_builtins();

    let mut data_stack_out = data_stack;
    let mut program_stack = program;
    let mut local_scope = HashMap::new();
    let mut scope_stack = Vec::new();
    while !program_stack.is_empty() {
        interpret_one(
            &mut data_stack_out,
            &mut program_stack,
            &mut local_scope,
            &mut scope_stack,
            &mut global_state,
            &builtins,
        );
    }
    match data_stack_out.as_slice() {
        [TemplateValue::Literal(ProgramValue::Data(DataValue::Float(v)))] => {
            assert!(
                (v - 5.0).abs() < 1e-9,
                "hypot({{x:3,y:4}}) should be ~5.0, got {}",
                v
            );
        }
        other => panic!("expected [Float(~5.0)], got {:?}", other),
    }
}

// ── c_load_header builtin tests ───────────────────────────────────────────────

#[test]
fn c_load_header_str_arg_derives_lib_name() {
    let builtins = get_interpret_builtins();
    // Pass just the path — lib_name should be derived from the filename stem ("test").
    let mut scope = HashMap::new();
    let mut state = GlobalState::new(&mut scope);
    let mut result = builtins["c_load_header"]
        .execute(
            &mut vec![sid::TemplateValue::Literal(sid::ProgramValue::Data(
                DataValue::Str(std::ffi::CString::new(fixture_header()).unwrap()),
            ))],
            &mut state,
            &mut vec![],
            &mut HashMap::new(),
            &builtins,
        )
        .expect("c_load_header failed");

    assert_eq!(result.len(), 1);
    match result.remove(0) {
        DataValue::Map(entries) => {
            let sqrt = entries
                .iter()
                .find(|(k, _)| matches!(k, DataValue::Label(n) if n == "sqrt"))
                .expect("sqrt not found");
            match &sqrt.1 {
                DataValue::CFuncSig(s) => assert_eq!(s.lib_name, "test"),
                other => panic!("expected CFuncSig, got {:?}", other),
            }
        }
        other => panic!("expected Map, got {:?}", other),
    }
}

#[test]
fn c_load_header_list_arg_uses_explicit_lib_name() {
    let builtins = get_interpret_builtins();
    let arg = DataValue::List(vec![
        DataValue::Str(std::ffi::CString::new(fixture_header()).unwrap()),
        DataValue::Str(std::ffi::CString::new(TEST_LIB).unwrap()),
    ]);

    let mut scope = HashMap::new();
    let mut state = GlobalState::new(&mut scope);
    let mut result = builtins["c_load_header"]
        .execute(
            &mut vec![sid::TemplateValue::Literal(sid::ProgramValue::Data(arg))],
            &mut state,
            &mut vec![],
            &mut HashMap::new(),
            &builtins,
        )
        .expect("c_load_header failed");

    assert_eq!(result.len(), 1);
    match result.remove(0) {
        DataValue::Map(entries) => {
            let sqrt = entries
                .iter()
                .find(|(k, _)| matches!(k, DataValue::Label(n) if n == "sqrt"))
                .expect("sqrt not found");
            match &sqrt.1 {
                DataValue::CFuncSig(s) => assert_eq!(s.lib_name, TEST_LIB),
                other => panic!("expected CFuncSig, got {:?}", other),
            }
        }
        other => panic!("expected Map, got {:?}", other),
    }
}

#[test]
fn c_load_header_error_on_wrong_arg() {
    let builtins = get_interpret_builtins();
    let mut scope = HashMap::new();
    let mut state = GlobalState::new(&mut scope);
    let result = builtins["c_load_header"].execute(
        &mut vec![sid::TemplateValue::Literal(sid::ProgramValue::Data(
            DataValue::Int(42),
        ))],
        &mut state,
        &mut vec![],
        &mut HashMap::new(),
        &builtins,
    );
    assert!(result.is_err());
}

// ── c_link_lib builtin tests ──────────────────────────────────────────────────

#[test]
fn c_link_lib_preloads_library() {
    let builtins = get_interpret_builtins();
    let mut scope = HashMap::new();
    let mut state = GlobalState::new(&mut scope);
    builtins["c_link_lib"]
        .execute(
            &mut vec![sid::TemplateValue::Literal(sid::ProgramValue::Data(
                DataValue::Str(std::ffi::CString::new(TEST_LIB).unwrap()),
            ))],
            &mut state,
            &mut vec![],
            &mut HashMap::new(),
            &builtins,
        )
        .expect("c_link_lib failed");
    assert!(
        state.libraries.contains_key(TEST_LIB),
        "library should be in central store"
    );
}

#[test]
fn c_link_lib_list_arg_registers_under_name() {
    let builtins = get_interpret_builtins();
    let mut scope = HashMap::new();
    let mut state = GlobalState::new(&mut scope);
    let arg = DataValue::List(vec![
        DataValue::Str(std::ffi::CString::new(TEST_LIB).unwrap()),
        DataValue::Str(std::ffi::CString::new("math").unwrap()),
    ]);
    builtins["c_link_lib"]
        .execute(
            &mut vec![sid::TemplateValue::Literal(sid::ProgramValue::Data(arg))],
            &mut state,
            &mut vec![],
            &mut HashMap::new(),
            &builtins,
        )
        .expect("c_link_lib with list failed");
    assert!(
        state.libraries.contains_key("math"),
        "library should be registered under 'math'"
    );
    assert!(
        !state.libraries.contains_key(TEST_LIB),
        "should not be registered under path"
    );
}

#[test]
fn c_link_lib_error_on_missing_library() {
    let builtins = get_interpret_builtins();
    let mut scope = HashMap::new();
    let mut state = GlobalState::new(&mut scope);
    let result = builtins["c_link_lib"].execute(
        &mut vec![sid::TemplateValue::Literal(sid::ProgramValue::Data(
            DataValue::Str(std::ffi::CString::new("/nonexistent/lib.so").unwrap()),
        ))],
        &mut state,
        &mut vec![],
        &mut HashMap::new(),
        &builtins,
    );
    assert!(result.is_err());
}

#[test]
fn c_link_lib_error_on_wrong_arg() {
    let builtins = get_interpret_builtins();
    let mut scope = HashMap::new();
    let mut state = GlobalState::new(&mut scope);
    let result = builtins["c_link_lib"].execute(
        &mut vec![sid::TemplateValue::Literal(sid::ProgramValue::Data(
            DataValue::Int(42),
        ))],
        &mut state,
        &mut vec![],
        &mut HashMap::new(),
        &builtins,
    );
    assert!(result.is_err());
}

// ── drop builtin tests ────────────────────────────────────────────────────────

#[test]
fn drop_discards_value() {
    let builtins = get_interpret_builtins();
    let mut scope = HashMap::new();
    let mut state = GlobalState::new(&mut scope);
    let result = builtins["drop"]
        .execute(
            &mut vec![sid::TemplateValue::Literal(sid::ProgramValue::Data(
                DataValue::Int(99),
            ))],
            &mut state,
            &mut vec![],
            &mut HashMap::new(),
            &builtins,
        )
        .expect("drop failed");
    assert!(result.is_empty(), "drop should return nothing");
}

// ── eq builtin tests ──────────────────────────────────────────────────────────

#[test]
fn eq_equal_values_returns_true() {
    let builtins = get_interpret_builtins();
    let mut scope = HashMap::new();
    let mut state = GlobalState::new(&mut scope);
    let result = builtins["eq"]
        .execute(
            &mut vec![
                sid::TemplateValue::Literal(sid::ProgramValue::Data(DataValue::Int(3))),
                sid::TemplateValue::Literal(sid::ProgramValue::Data(DataValue::Int(3))),
            ],
            &mut state,
            &mut vec![],
            &mut HashMap::new(),
            &builtins,
        )
        .expect("eq failed");
    assert_eq!(result, vec![DataValue::Bool(true)]);
}

#[test]
fn eq_unequal_values_returns_false() {
    let builtins = get_interpret_builtins();
    let mut scope = HashMap::new();
    let mut state = GlobalState::new(&mut scope);
    let result = builtins["eq"]
        .execute(
            &mut vec![
                sid::TemplateValue::Literal(sid::ProgramValue::Data(DataValue::Int(1))),
                sid::TemplateValue::Literal(sid::ProgramValue::Data(DataValue::Int(2))),
            ],
            &mut state,
            &mut vec![],
            &mut HashMap::new(),
            &builtins,
        )
        .expect("eq failed");
    assert_eq!(result, vec![DataValue::Bool(false)]);
}

#[test]
fn eq_error_on_wrong_arg() {
    let builtins = get_interpret_builtins();
    let mut scope = HashMap::new();
    let mut state = GlobalState::new(&mut scope);
    // Stack with only one value — should error
    let result = builtins["eq"].execute(
        &mut vec![sid::TemplateValue::Literal(sid::ProgramValue::Data(
            DataValue::Int(1),
        ))],
        &mut state,
        &mut vec![],
        &mut HashMap::new(),
        &builtins,
    );
    assert!(result.is_err());
}

// ── assert builtin tests ──────────────────────────────────────────────────────

#[test]
fn assert_passes_on_true() {
    let builtins = get_interpret_builtins();
    let mut scope = HashMap::new();
    let mut state = GlobalState::new(&mut scope);
    let result = builtins["assert"]
        .execute(
            &mut vec![sid::TemplateValue::Literal(sid::ProgramValue::Data(
                DataValue::Bool(true),
            ))],
            &mut state,
            &mut vec![],
            &mut HashMap::new(),
            &builtins,
        )
        .expect("assert should not fail on true");
    assert!(result.is_empty());
}

#[test]
fn assert_errors_on_false() {
    let builtins = get_interpret_builtins();
    let mut scope = HashMap::new();
    let mut state = GlobalState::new(&mut scope);
    let result = builtins["assert"].execute(
        &mut vec![sid::TemplateValue::Literal(sid::ProgramValue::Data(
            DataValue::Bool(false),
        ))],
        &mut state,
        &mut vec![],
        &mut HashMap::new(),
        &builtins,
    );
    assert!(result.is_err());
}

#[test]
fn assert_error_on_non_bool() {
    let builtins = get_interpret_builtins();
    let mut scope = HashMap::new();
    let mut state = GlobalState::new(&mut scope);
    let result = builtins["assert"].execute(
        &mut vec![sid::TemplateValue::Literal(sid::ProgramValue::Data(
            DataValue::Int(1),
        ))],
        &mut state,
        &mut vec![],
        &mut HashMap::new(),
        &builtins,
    );
    assert!(result.is_err());
}

// ── c_load_header is comptime-available ───────────────────────────────────────

#[test]
fn c_load_header_available_at_comptime() {
    let builtins = get_comptime_builtins();
    // must be present
    for name in &[
        "c_load_header",
        "load_scope",
        "drop",
        "eq",
        "assert",
        "not",
        "ptr_cast",
        "debug_stack",
    ] {
        assert!(
            builtins.contains_key(name),
            "{name} must be a comptime builtin"
        );
    }
    // must NOT be present
    for name in &["c_link_lib", "ptr_read_cstr"] {
        assert!(
            !builtins.contains_key(name),
            "{name} must NOT be a comptime builtin"
        );
    }
}

// ── load_scope builtin tests ──────────────────────────────────────────────────

#[test]
fn load_scope_inserts_struct_fields_into_global_scope() {
    let builtins = get_interpret_builtins();
    let mut scope = HashMap::new();
    let mut state = GlobalState::new(&mut scope);
    let arg = DataValue::Map(vec![
        (
            DataValue::Label("sqrt".to_owned()),
            DataValue::CFuncSig(get_sqrt_sig()),
        ),
        (DataValue::Label("answer".to_owned()), DataValue::Int(42)),
    ]);
    let result = builtins["load_scope"]
        .execute(
            &mut vec![sid::TemplateValue::Literal(sid::ProgramValue::Data(arg))],
            &mut state,
            &mut vec![],
            &mut HashMap::new(),
            &builtins,
        )
        .expect("load_scope failed");
    assert!(result.is_empty(), "load_scope should return nothing");
    assert!(state.scope.contains_key("sqrt"), "sqrt should be in scope");
    assert_eq!(state.scope.get("answer"), Some(&DataValue::Int(42)));
}

#[test]
fn load_scope_error_on_non_struct() {
    let builtins = get_interpret_builtins();
    let mut scope = HashMap::new();
    let mut state = GlobalState::new(&mut scope);
    let result = builtins["load_scope"].execute(
        &mut vec![sid::TemplateValue::Literal(sid::ProgramValue::Data(
            DataValue::Int(1),
        ))],
        &mut state,
        &mut vec![],
        &mut HashMap::new(),
        &builtins,
    );
    assert!(result.is_err());
}
