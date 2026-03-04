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
fn skips_variadic_functions() {
    let sigs = parse_c_header(&fixture_header(), TEST_LIB).expect("parse_c_header failed");
    assert!(
        sigs.iter().all(|s| s.name != "printf"),
        "printf (variadic) should have been skipped"
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
    let sig = get_sqrt_sig();
    let mut libs = HashMap::new();
    let result = call_cfuncsig(&sig, Some(DataValue::Float(9.0)), &mut libs)
        .expect("call_cfuncsig failed");
    match result {
        Some(DataValue::Float(v)) => {
            assert!((v - 3.0).abs() < 1e-9, "sqrt(9.0) should be ~3.0, got {}", v);
        }
        other => panic!("expected DataValue::Float, got {:?}", other),
    }
    // Library should now be cached in the map.
    assert!(libs.contains_key(TEST_LIB));
}

#[test]
fn call_with_wrong_arg_count_errors() {
    let sig = get_sqrt_sig();
    let result = call_cfuncsig(&sig, None, &mut HashMap::new());
    assert!(result.is_err(), "wrong arg count should return Err");
}

#[test]
fn error_on_nonexistent_library() {
    let sigs = parse_c_header(&fixture_header(), "/nonexistent/libxyz.so.999")
        .expect("parse succeeds regardless of lib path");
    let result = call_cfuncsig(&sigs[0], None, &mut HashMap::new());
    assert!(result.is_err(), "should error when library cannot be opened");
}

// ── CFuncSig invoke via interpret ─────────────────────────────────────────────

#[test]
fn interpret_cfuncsig_in_global_scope() {
    let sqrt_sig = get_sqrt_sig();

    let mut global_state = GlobalState::new();
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
fn c_load_header_returns_struct_of_cfuncsigs() {
    let builtins = get_interpret_builtins();
    let arg = DataValue::Struct(vec![
        ("header".to_owned(), DataValue::Str(fixture_header())),
        ("lib".to_owned(), DataValue::Str(TEST_LIB.to_owned())),
    ]);

    let result = builtins["c_load_header"]
        .execute(Some(arg), &mut GlobalState::new())
        .expect("c_load_header failed");

    match result {
        Some(DataValue::Struct(fields)) => {
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
    let result = builtins["c_load_header"]
        .execute(Some(DataValue::Str("not-a-struct".to_owned())), &mut GlobalState::new());
    assert!(result.is_err());
}

// ── c_link_lib builtin tests ──────────────────────────────────────────────────

#[test]
fn c_link_lib_preloads_library() {
    let builtins = get_interpret_builtins();
    let mut state = GlobalState::new();
    builtins["c_link_lib"]
        .execute(Some(DataValue::Str(TEST_LIB.to_owned())), &mut state)
        .expect("c_link_lib failed");
    assert!(state.libraries.contains_key(TEST_LIB), "library should be in central store");
}

#[test]
fn c_link_lib_error_on_missing_library() {
    let builtins = get_interpret_builtins();
    let result = builtins["c_link_lib"]
        .execute(Some(DataValue::Str("/nonexistent/lib.so".to_owned())), &mut GlobalState::new());
    assert!(result.is_err());
}

#[test]
fn c_link_lib_error_on_wrong_arg() {
    let builtins = get_interpret_builtins();
    let result = builtins["c_link_lib"]
        .execute(Some(DataValue::Int(42)), &mut GlobalState::new());
    assert!(result.is_err());
}

// ── c_load_header is comptime-available ───────────────────────────────────────

#[test]
fn c_load_header_available_at_comptime() {
    let builtins = get_comptime_builtins();
    assert!(builtins.contains_key("c_load_header"), "c_load_header must be a comptime builtin");
    assert!(!builtins.contains_key("c_link_lib"), "c_link_lib must NOT be a comptime builtin");
}


