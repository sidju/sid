use crate::built_in::BuiltinEntry;
use crate::c_ffi::{open_library, parse_c_header};
use crate::type_system::SidType;
use crate::DataValue;

fn pop_arg(args: &mut Vec<DataValue>, name: &str) -> DataValue {
    args.pop()
        .unwrap_or_else(|| panic!("{}: expected an argument", name))
}

fn cstring_to_string(cs: std::ffi::CString) -> String {
    cs.into_string()
        .unwrap_or_else(|e| e.into_cstring().to_string_lossy().into_owned())
}

fn stem_of(path: &str) -> String {
    std::path::Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.to_owned())
        .unwrap_or_else(|| "unknown".to_owned())
}

pub fn c_load_header() -> BuiltinEntry {
    BuiltinEntry {
        name: "c_load_header",
        args: vec![SidType::Any],
        ret: vec![SidType::Any],
        exec: |_state, mut args| {
            let arg = pop_arg(&mut args, "c_load_header");
            let (header_path, lib_name) = match arg {
                DataValue::Str(path) => {
                    let lib_name = stem_of(&path.to_string_lossy());
                    (cstring_to_string(path), lib_name)
                }
                DataValue::List(mut items) if items.len() == 2 => {
                    let path = match items.remove(0) {
                        DataValue::Str(s) => cstring_to_string(s),
                        other => panic!(
                            "c_load_header: first list element must be Str, got {:?}",
                            other
                        ),
                    };
                    let lib_name = match items.remove(0) {
                        DataValue::Str(s) => cstring_to_string(s),
                        other => panic!(
                            "c_load_header: second list element must be Str, got {:?}",
                            other
                        ),
                    };
                    (path, lib_name)
                }
                other => panic!("c_load_header expects Str or [Str, Str], got {:?}", other),
            };
            let sigs = parse_c_header(&header_path, &lib_name).unwrap();
            let out_fields: Vec<(DataValue, DataValue)> = sigs
                .into_iter()
                .map(|s| (DataValue::Label(s.name.clone()), DataValue::CFuncSig(s)))
                .collect();
            vec![DataValue::Map(out_fields)]
        },
    }
}

pub fn c_link_lib() -> BuiltinEntry {
    BuiltinEntry {
        name: "c_link_lib",
        args: vec![SidType::Any],
        ret: vec![],
        exec: |state, mut args| {
            let arg = pop_arg(&mut args, "c_link_lib");
            let (lib_path, lib_name) = match arg {
                DataValue::Str(path) => {
                    let s = cstring_to_string(path.clone());
                    (s.clone(), s)
                }
                DataValue::List(mut items) if items.len() == 2 => {
                    let path = match items.remove(0) {
                        DataValue::Str(s) => cstring_to_string(s),
                        other => panic!(
                            "c_link_lib: first list element must be Str, got {:?}",
                            other
                        ),
                    };
                    let name = match items.remove(0) {
                        DataValue::Str(s) => cstring_to_string(s),
                        other => panic!(
                            "c_link_lib: second list element must be Str, got {:?}",
                            other
                        ),
                    };
                    (path, name)
                }
                other => panic!("c_link_lib expects Str or [Str, Str], got {:?}", other),
            };
            if !state.global_state.libraries.contains_key(lib_name.as_str()) {
                let lib = open_library(&lib_path).unwrap();
                state.global_state.libraries.insert(lib_name, lib);
            }
            vec![]
        },
    }
}

pub fn ptr_cast() -> BuiltinEntry {
    BuiltinEntry {
        name: "ptr_cast",
        args: vec![SidType::Any, SidType::Any],
        ret: vec![SidType::Any],
        exec: |_state, mut args| {
            let new_type = pop_arg(&mut args, "ptr_cast");
            let pointer = pop_arg(&mut args, "ptr_cast");
            let addr = match pointer {
                DataValue::Pointer { addr, .. } => addr,
                other => panic!(
                    "ptr_cast: first argument must be a Pointer, got {:?}",
                    other
                ),
            };
            let pointee_ty = match new_type {
                DataValue::Type(ty) => ty,
                other => panic!("ptr_cast: type argument must be a Type, got {:?}", other),
            };
            vec![DataValue::Pointer { addr, pointee_ty }]
        },
    }
}

pub fn ptr_read_cstr() -> BuiltinEntry {
    BuiltinEntry {
        name: "ptr_read_cstr",
        args: vec![SidType::Any],
        ret: vec![SidType::Str],
        exec: |_state, mut args| match pop_arg(&mut args, "ptr_read_cstr") {
            DataValue::Pointer { addr, .. } => {
                let ptr = addr as *const std::ffi::c_char;
                if ptr.is_null() {
                    panic!("ptr_read_cstr: pointer is null");
                }
                let cs = unsafe { std::ffi::CStr::from_ptr(ptr) }.to_owned();
                vec![DataValue::Str(cs)]
            }
            other => panic!("ptr_read_cstr expects Pointer, got {:?}", other),
        },
    }
}
