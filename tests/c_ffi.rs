use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use sid::*;

// ── Helper to locate test fixtures ───────────────────────────────────────────

fn fixture_header() -> String {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("tests/fixtures/test.h");
    p.to_string_lossy().into_owned()
}

// ── C header parsing tests ────────────────────────────────────────────────────

#[test]
fn parses_double_param_double_return() {
    let sigs = parse_c_header(&fixture_header()).expect("parse_c_header failed");
    let sqrt_sig = sigs.iter().find(|s| s.name == "sqrt").expect("sqrt not found");
    assert_eq!(sqrt_sig.ret, CType::Double);
    assert_eq!(sqrt_sig.params, vec![CType::Double]);
}

#[test]
fn parses_cstring_param_int_return() {
    let sigs = parse_c_header(&fixture_header()).expect("parse_c_header failed");
    let puts_sig = sigs.iter().find(|s| s.name == "puts").expect("puts not found");
    assert_eq!(puts_sig.ret, CType::Int);
    assert_eq!(puts_sig.params, vec![CType::CString]);
}

#[test]
fn skips_variadic_functions() {
    let sigs = parse_c_header(&fixture_header()).expect("parse_c_header failed");
    assert!(
        sigs.iter().all(|s| s.name != "printf"),
        "printf (variadic) should have been skipped"
    );
}

#[test]
fn skips_struct_and_typedef_declarations() {
    let sigs = parse_c_header(&fixture_header()).expect("parse_c_header failed");
    // Neither "Foo" nor "MyInt" should appear as a function name.
    for sig in &sigs {
        assert_ne!(sig.name, "Foo", "struct should be skipped");
        assert_ne!(sig.name, "MyInt", "typedef should be skipped");
    }
}

#[test]
fn error_on_nonexistent_header() {
    let result = parse_c_header("/nonexistent/path/header.h");
    assert!(result.is_err(), "should error on missing file");
}

// ── Library loading tests ─────────────────────────────────────────────────────

#[test]
fn loads_sqrt_from_libm() {
    let sigs = parse_c_header(&fixture_header()).expect("parse_c_header failed");
    let funcs = load_c_functions("libm.so.6", &sigs).expect("load_c_functions failed");
    assert!(
        funcs.iter().any(|f| f.name == "sqrt"),
        "sqrt should be present in libm"
    );
}

#[test]
fn load_skips_missing_symbols() {
    let sigs = vec![
        CFuncSig { name: "this_function_does_not_exist_xyz".to_owned(), ret: CType::Int, params: vec![] },
    ];
    let funcs = load_c_functions("libm.so.6", &sigs).expect("load_c_functions failed");
    assert!(funcs.is_empty(), "missing symbols should be silently skipped");
}

#[test]
fn error_on_nonexistent_library() {
    let result = load_c_functions("/nonexistent/libxyz.so.999", &[]);
    assert!(result.is_err(), "should error on missing library");
}

// ── call_c_function tests ─────────────────────────────────────────────────────

fn get_sqrt_func() -> CFunc {
    let sigs = parse_c_header(&fixture_header()).expect("parse_c_header failed");
    let funcs = load_c_functions("libm.so.6", &sigs).expect("load_c_functions failed");
    funcs.into_iter().find(|f| f.name == "sqrt").expect("sqrt not found in libm")
}

#[test]
fn call_sqrt_returns_correct_value() {
    let sqrt_fn = get_sqrt_func();
    let result = call_c_function(&sqrt_fn, Some(DataValue::Float(9.0)))
        .expect("call_c_function failed");
    match result {
        Some(DataValue::Float(v)) => {
            assert!((v - 3.0).abs() < 1e-9, "sqrt(9.0) should be ~3.0, got {}", v);
        }
        other => panic!("expected DataValue::Float, got {:?}", other),
    }
}

#[test]
fn call_with_wrong_arg_count_errors() {
    let sqrt_fn = get_sqrt_func();
    // sqrt takes 1 param; pass nothing.
    let result = call_c_function(&sqrt_fn, None);
    assert!(result.is_err(), "wrong arg count should return Err");
}

// ── CFunction invoke via interpret ────────────────────────────────────────────

#[test]
fn interpret_cfunction_in_global_scope() {
    let sqrt_fn = get_sqrt_func();

    // Put the CFunction in global scope under the name "sqrt".
    let mut global_scope: HashMap<String, DataValue> = HashMap::new();
    global_scope.insert("sqrt".to_owned(), DataValue::CFunction(Arc::new(sqrt_fn)));

    // Program: push 16.0, resolve label "sqrt", invoke.
    let program = vec![ProgramValue::Invoke];
    let data_stack = vec![
        DataValue::Float(16.0).into(),
        DataValue::Label("sqrt".to_owned()).into(),
    ];
    let builtins = get_interpret_builtins();

    interpret(program, data_stack, global_scope, &builtins);
    // If we get here without panicking, the call succeeded.
    // The result (4.0) was pushed onto the stack but we can't inspect it
    // from outside `interpret`. A panicking test would fail above.
}

// ── c_import builtin tests ────────────────────────────────────────────────────

#[test]
fn c_import_returns_struct_of_cfunctions() {
    let builtins = get_interpret_builtins();
    let c_import = builtins["c_import"];

    let arg = DataValue::Struct(vec![
        ("header".to_owned(), DataValue::Str(fixture_header())),
        ("lib".to_owned(), DataValue::Str("libm.so.6".to_owned())),
    ]);

    let result = c_import.execute(Some(arg), &HashMap::new())
        .expect("c_import failed");

    match result {
        Some(DataValue::Struct(fields)) => {
            let has_sqrt = fields.iter().any(|(name, val)| {
                name == "sqrt" && matches!(val, DataValue::CFunction(_))
            });
            assert!(has_sqrt, "returned struct should contain a CFunction named 'sqrt'");
        }
        other => panic!("expected Struct, got {:?}", other),
    }
}

#[test]
fn c_import_error_on_wrong_arg_type() {
    let builtins = get_interpret_builtins();
    let c_import = builtins["c_import"];

    let result = c_import.execute(Some(DataValue::Str("not-a-struct".to_owned())), &HashMap::new());
    assert!(result.is_err(), "c_import with non-struct arg should return Err");
}
