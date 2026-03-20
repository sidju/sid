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
    let sqrt_sig = sigs.iter().find(|s| s.name == "sqrt").expect("sqrt not found");
    assert_eq!(sqrt_sig.ret, CType::Double);
    assert_eq!(sqrt_sig.params, vec![CType::Double]);
    assert_eq!(sqrt_sig.lib_name, TEST_LIB);
}

#[test]
fn parses_cstring_param_int_return() {
    let sigs = parse_c_header(&fixture_header(), TEST_LIB).expect("parse_c_header failed");
    let puts_sig = sigs.iter().find(|s| s.name == "puts").expect("puts not found");
    assert_eq!(puts_sig.ret, CType::Int);
    assert_eq!(puts_sig.params, vec![CType::CString]);
}

#[test]
fn includes_variadic_functions() {
    let sigs = parse_c_header(&fixture_header(), TEST_LIB).expect("parse_c_header failed");
    let printf = sigs.iter().find(|s| s.name == "printf")
        .expect("printf (variadic) should be included");
    assert!(printf.variadic, "printf should be marked variadic");
    assert_eq!(printf.params.len(), 1, "printf should have 1 fixed param (the format string)");
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
    libs.insert(TEST_LIB.to_owned(), sid::c_ffi_open_library(TEST_LIB).expect("open libm"));

    let sig = get_sqrt_sig();
    let result = call_cfuncsig(&sig, Some(DataValue::Float(9.0)), &libs)
        .expect("call_cfuncsig failed");
    match result {
        Some(DataValue::Float(v)) => {
            assert!((v - 3.0).abs() < 1e-9, "sqrt(9.0) should be ~3.0, got {}", v);
        }
        other => panic!("expected DataValue::Float, got {:?}", other),
    }
}

#[test]
fn call_with_wrong_arg_count_errors() {
    let mut libs = HashMap::new();
    libs.insert(TEST_LIB.to_owned(), sid::c_ffi_open_library(TEST_LIB).expect("open libm"));
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
    global_state.scope.insert("sqrt".to_owned(), DataValue::CFuncSig(sqrt_sig));

    let program = vec![ProgramValue::Invoke];
    let data_stack = vec![
        DataValue::Float(16.0).into(),
        DataValue::Label("sqrt".to_owned()).into(),
    ];
    let builtins = get_interpret_builtins();

    interpret(program, data_stack, global_state, &builtins);
}

// ── c_load_header builtin tests ───────────────────────────────────────────────

#[test]
fn c_load_header_str_arg_derives_lib_name() {
    let builtins = get_interpret_builtins();
    // Pass just the path — lib_name should be derived from the filename stem ("test").
    let mut scope = HashMap::new();
    let mut state = GlobalState::new(&mut scope);
    let mut result = builtins["c_load_header"]
        .execute(Some(DataValue::Str(std::ffi::CString::new(fixture_header()).unwrap())), &mut state)
        .expect("c_load_header failed");

    assert_eq!(result.len(), 1);
    match result.remove(0) {
        DataValue::Struct(fields) => {
            let sqrt = fields.iter()
                .find(|(name, _)| name == "sqrt")
                .expect("sqrt not found");
            match &sqrt.1 {
                DataValue::CFuncSig(s) => assert_eq!(s.lib_name, "test"),
                other => panic!("expected CFuncSig, got {:?}", other),
            }
        }
        other => panic!("expected Struct, got {:?}", other),
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
        .execute(Some(arg), &mut state)
        .expect("c_load_header failed");

    assert_eq!(result.len(), 1);
    match result.remove(0) {
        DataValue::Struct(fields) => {
            let sqrt = fields.iter()
                .find(|(name, _)| name == "sqrt")
                .expect("sqrt not found");
            match &sqrt.1 {
                DataValue::CFuncSig(s) => assert_eq!(s.lib_name, TEST_LIB),
                other => panic!("expected CFuncSig, got {:?}", other),
            }
        }
        other => panic!("expected Struct, got {:?}", other),
    }
}

#[test]
fn c_load_header_error_on_wrong_arg() {
    let builtins = get_interpret_builtins();
    let mut scope = HashMap::new();
    let mut state = GlobalState::new(&mut scope);
    let result = builtins["c_load_header"]
        .execute(Some(DataValue::Int(42)), &mut state);
    assert!(result.is_err());
}

// ── c_link_lib builtin tests ──────────────────────────────────────────────────

#[test]
fn c_link_lib_preloads_library() {
    let builtins = get_interpret_builtins();
    let mut scope = HashMap::new();
    let mut state = GlobalState::new(&mut scope);
    builtins["c_link_lib"]
        .execute(Some(DataValue::Str(std::ffi::CString::new(TEST_LIB).unwrap())), &mut state)
        .expect("c_link_lib failed");
    assert!(state.libraries.contains_key(TEST_LIB), "library should be in central store");
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
        .execute(Some(arg), &mut state)
        .expect("c_link_lib with list failed");
    assert!(state.libraries.contains_key("math"), "library should be registered under 'math'");
    assert!(!state.libraries.contains_key(TEST_LIB), "should not be registered under path");
}

#[test]
fn c_link_lib_error_on_missing_library() {
    let builtins = get_interpret_builtins();
    let mut scope = HashMap::new();
    let mut state = GlobalState::new(&mut scope);
    let result = builtins["c_link_lib"]
        .execute(Some(DataValue::Str(std::ffi::CString::new("/nonexistent/lib.so").unwrap())), &mut state);
    assert!(result.is_err());
}

#[test]
fn c_link_lib_error_on_wrong_arg() {
    let builtins = get_interpret_builtins();
    let mut scope = HashMap::new();
    let mut state = GlobalState::new(&mut scope);
    let result = builtins["c_link_lib"]
        .execute(Some(DataValue::Int(42)), &mut state);
    assert!(result.is_err());
}

// ── drop builtin tests ────────────────────────────────────────────────────────

#[test]
fn drop_discards_value() {
    let builtins = get_interpret_builtins();
    let mut scope = HashMap::new();
    let mut state = GlobalState::new(&mut scope);
    let result = builtins["drop"]
        .execute(Some(DataValue::Int(99)), &mut state)
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
        .execute(Some(DataValue::List(vec![DataValue::Int(3), DataValue::Int(3)])), &mut state)
        .expect("eq failed");
    assert_eq!(result, vec![DataValue::Bool(true)]);
}

#[test]
fn eq_unequal_values_returns_false() {
    let builtins = get_interpret_builtins();
    let mut scope = HashMap::new();
    let mut state = GlobalState::new(&mut scope);
    let result = builtins["eq"]
        .execute(Some(DataValue::List(vec![DataValue::Int(1), DataValue::Int(2)])), &mut state)
        .expect("eq failed");
    assert_eq!(result, vec![DataValue::Bool(false)]);
}

#[test]
fn eq_error_on_wrong_arg() {
    let builtins = get_interpret_builtins();
    let mut scope = HashMap::new();
    let mut state = GlobalState::new(&mut scope);
    let result = builtins["eq"].execute(Some(DataValue::Int(1)), &mut state);
    assert!(result.is_err());
}

// ── assert builtin tests ──────────────────────────────────────────────────────

#[test]
fn assert_passes_on_true() {
    let builtins = get_interpret_builtins();
    let mut scope = HashMap::new();
    let mut state = GlobalState::new(&mut scope);
    let result = builtins["assert"]
        .execute(Some(DataValue::Bool(true)), &mut state)
        .expect("assert should not fail on true");
    assert!(result.is_empty());
}

#[test]
fn assert_errors_on_false() {
    let builtins = get_interpret_builtins();
    let mut scope = HashMap::new();
    let mut state = GlobalState::new(&mut scope);
    let result = builtins["assert"].execute(Some(DataValue::Bool(false)), &mut state);
    assert!(result.is_err());
}

#[test]
fn assert_error_on_non_bool() {
    let builtins = get_interpret_builtins();
    let mut scope = HashMap::new();
    let mut state = GlobalState::new(&mut scope);
    let result = builtins["assert"].execute(Some(DataValue::Int(1)), &mut state);
    assert!(result.is_err());
}

// ── c_load_header is comptime-available ───────────────────────────────────────

#[test]
fn c_load_header_available_at_comptime() {
    let builtins = get_comptime_builtins();
    assert!(builtins.contains_key("c_load_header"), "c_load_header must be a comptime builtin");
    assert!(builtins.contains_key("load_scope"), "load_scope must be a comptime builtin");
    assert!(!builtins.contains_key("c_link_lib"), "c_link_lib must NOT be a comptime builtin");
    assert!(!builtins.contains_key("clone"), "clone must NOT be a comptime builtin");
}

// ── load_scope builtin tests ──────────────────────────────────────────────────

#[test]
fn load_scope_inserts_struct_fields_into_global_scope() {
    let builtins = get_interpret_builtins();
    let mut scope = HashMap::new();
    let mut state = GlobalState::new(&mut scope);
    let arg = DataValue::Struct(vec![
        ("sqrt".to_owned(), DataValue::CFuncSig(get_sqrt_sig())),
        ("answer".to_owned(), DataValue::Int(42)),
    ]);
    let result = builtins["load_scope"]
        .execute(Some(arg), &mut state)
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
    let result = builtins["load_scope"]
        .execute(Some(DataValue::Int(1)), &mut state);
    assert!(result.is_err());
}

// ── clone builtin tests ───────────────────────────────────────────────────────

#[test]
fn clone_duplicates_value() {
    let builtins = get_interpret_builtins();
    let mut scope = HashMap::new();
    let mut state = GlobalState::new(&mut scope);
    let result = builtins["clone"]
        .execute(Some(DataValue::Int(7)), &mut state)
        .expect("clone failed");
    assert_eq!(result, vec![DataValue::Int(7), DataValue::Int(7)]);
}

#[test]
fn clone_error_on_no_value() {
    let builtins = get_interpret_builtins();
    let mut scope = HashMap::new();
    let mut state = GlobalState::new(&mut scope);
    let result = builtins["clone"].execute(None, &mut state);
    assert!(result.is_err());
}


