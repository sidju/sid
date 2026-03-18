//! C header parsing via the system C preprocessor + lang-c.

use anyhow::Result;

use super::types::{CType, CFuncSig};
use crate::type_system::SidType;

/// Parse a C header file and return all bridgeable function signatures.
///
/// `lib_name` is stored on every returned [`CFuncSig`] so that callers can
/// invoke the functions without a separate link step.
///
/// The file is processed through the system C preprocessor (`gcc -E` on Linux,
/// `clang -E` on macOS) so that `#include` guards, macros, and transitive
/// includes are fully resolved before parsing.  Variadic functions, function
/// pointers, struct/union/enum/typedef declarations, and any declaration whose
/// types cannot be bridged are silently skipped.
pub fn parse_c_header(path: &str, lib_name: &str) -> Result<Vec<CFuncSig>> {
    let mut config = lang_c::driver::Config::default();
    // Strip GCC-extension keywords and attributes that lang-c doesn't understand.
    // __attribute__((malloc(fclose, ...))) and similar annotations cause lang-c
    // to silently drop entire function declarations (e.g. fopen on glibc systems).
    config.cpp_options.extend([
        "-D__restrict=".to_owned(),
        "-D__restrict__=".to_owned(),
        "-D__attribute__(x)=".to_owned(),
    ]);
    let parse = lang_c::driver::parse(&config, path)
        .map_err(|e| anyhow::anyhow!("failed to parse '{}': {}", path, e))?;
    Ok(extract_function_sigs(&parse.unit, lib_name))
}

/// Walk a fully preprocessed translation unit and return bridgeable function
/// signatures.
fn extract_function_sigs(unit: &lang_c::ast::TranslationUnit, lib_name: &str) -> Vec<CFuncSig> {
    unit.0.iter().filter_map(|ext| {
        if let lang_c::ast::ExternalDeclaration::Declaration(decl) = &ext.node {
            try_extract_func_sig(&decl.node, lib_name)
        } else {
            None
        }
    }).collect()
}

/// Attempt to extract a bridgeable function signature from a top-level
/// declaration.  Returns `None` for anything that is not a simple function.
fn try_extract_func_sig(decl: &lang_c::ast::Declaration, lib_name: &str) -> Option<CFuncSig> {
    use lang_c::ast::*;

    // Work with the first (and usually only) declarator in the declaration.
    let init_decl = decl.declarators.first()?;
    let declarator = &init_decl.node.declarator.node;

    // Only plain identifiers are function names we can bridge; skip nested
    // declarators like pointer-to-function `(*fn_ptr)(…)`.
    let name = match &declarator.kind.node {
        DeclaratorKind::Identifier(id) => id.node.name.clone(),
        _ => return None,
    };

    // Find the Function derived declarator. For a plain function `T f(params)`,
    // derived = [Function]. For a function returning a pointer `T *f(params)`,
    // lang_c puts derived = [Pointer, Function] — the Pointer comes first
    // because it is the "outer" modifier in the C grammar.
    let func_idx = declarator.derived.iter().position(|d| {
        matches!(d.node, DerivedDeclarator::Function(_))
    })?;
    let func_decl = match &declarator.derived[func_idx].node {
        DerivedDeclarator::Function(f) => &f.node,
        _ => unreachable!(),
    };

    // Note whether the function is variadic. We still bridge it — the caller
    // passes a List that includes both the fixed params and the variadic args,
    // whose C types are inferred from the runtime DataValues.
    let variadic = func_decl.ellipsis == Ellipsis::Some;

    // Any Pointer derived declarators that appear before the Function entry
    // (i.e. at indices < func_idx) indicate a pointer return type.
    let has_return_ptr = declarator.derived[..func_idx].iter().any(|d| {
        matches!(&d.node, DerivedDeclarator::Pointer(_))
    });

    let ret = specifiers_to_ctype(&decl.specifiers, has_return_ptr)?;
    let params = extract_params(&func_decl.parameters)?;

    Some(CFuncSig { name, ret, params, variadic, lib_name: lib_name.to_owned() })
}

/// Extract the parameter types from a function's parameter list.
///
/// Returns `None` if any parameter type cannot be bridged.
fn extract_params(
    params: &[lang_c::span::Node<lang_c::ast::ParameterDeclaration>],
) -> Option<Vec<CType>> {
    use lang_c::ast::*;

    if params.is_empty() {
        return Some(vec![]);
    }

    // A single `void` parameter means the function takes no arguments.
    if params.len() == 1 {
        let p = &params[0].node;
        let is_void_only = p.declarator.is_none()
            && p.specifiers.iter().any(|s| {
                matches!(
                    &s.node,
                    DeclarationSpecifier::TypeSpecifier(ts)
                    if matches!(ts.node, TypeSpecifier::Void)
                )
            });
        if is_void_only {
            return Some(vec![]);
        }
    }

    let mut result = Vec::new();
    for param_node in params {
        let param = &param_node.node;

        // Is there a pointer in the parameter's declarator?
        let has_ptr = param.declarator.as_ref().map(|d| {
            d.node.derived.iter().any(|der| {
                matches!(&der.node, DerivedDeclarator::Pointer(_))
            })
        }).unwrap_or(false);

        let ctype = specifiers_to_ctype(&param.specifiers, has_ptr)?;
        if ctype != CType::Void {
            result.push(ctype);
        }
    }
    Some(result)
}

/// Map a set of C declaration specifiers (plus a pointer flag) to a [`CType`].
///
/// Returns `None` if the type combination cannot be bridged.
fn specifiers_to_ctype(
    specs: &[lang_c::span::Node<lang_c::ast::DeclarationSpecifier>],
    has_ptr: bool,
) -> Option<CType> {
    use lang_c::ast::{DeclarationSpecifier, TypeSpecifier};

    // Collect just the TypeSpecifier variants; ignore qualifiers, storage class, etc.
    let type_specs: Vec<&TypeSpecifier> = specs.iter().filter_map(|s| {
        if let DeclarationSpecifier::TypeSpecifier(ts) = &s.node {
            Some(&ts.node)
        } else {
            None
        }
    }).collect();

    let has_char   = type_specs.iter().any(|s| matches!(s, TypeSpecifier::Char));
    let has_void   = type_specs.iter().any(|s| matches!(s, TypeSpecifier::Void));
    let has_float  = type_specs.iter().any(|s| matches!(s, TypeSpecifier::Float));
    let has_double = type_specs.iter().any(|s| matches!(s, TypeSpecifier::Double));
    let has_short  = type_specs.iter().any(|s| matches!(s, TypeSpecifier::Short));
    let has_int    = type_specs.iter().any(|s| matches!(s, TypeSpecifier::Int));
    let long_count = type_specs.iter().filter(|s| matches!(s, TypeSpecifier::Long)).count();

    // `char *` → CString; bare `char` → Int.
    if has_char {
        return if has_ptr { Some(CType::CString) } else { Some(CType::Int) };
    }

    // Any other pointer — determine the pointee SidType for display.
    if has_ptr {
        let pointee = if has_void {
            SidType::Any
        } else if has_double || has_float {
            SidType::Float
        } else if has_int || has_short || long_count > 0 {
            SidType::Int
        } else {
            // Typedef or unknown base type — fall back to Any
            typedef_name(&type_specs)
                .and_then(sid_type_for_typedef)
                .unwrap_or(SidType::Any)
        };
        return Some(CType::Pointer(pointee));
    }

    // Non-pointer primitive types.
    if has_void   { return Some(CType::Void); }
    if has_double { return Some(CType::Double); }
    if has_float  { return Some(CType::Float); }
    if long_count >= 2                     { return Some(CType::Long); } // long long
    if long_count == 1 && !has_int         { return Some(CType::Long); } // bare long
    if has_int || has_short || long_count > 0 { return Some(CType::Int); }

    // Typedef names: size_t, int32_t, etc.
    if let Some(name) = typedef_name(&type_specs) {
        return match name {
            "size_t" => Some(CType::SizeT),
            "ssize_t" | "ptrdiff_t" | "intmax_t" | "uintmax_t"
            | "intptr_t" | "uintptr_t" => Some(CType::Long),
            "uint8_t" | "uint16_t" | "uint32_t" | "uint64_t"
            | "int8_t"  | "int16_t"  | "int32_t"  | "int64_t"
            | "off_t" | "pid_t" | "uid_t" | "gid_t" | "mode_t"
            | "dev_t" | "ino_t" | "nlink_t" | "socklen_t" => Some(CType::Int),
            _ => None,
        };
    }

    None // unknown type — caller skips this function
}

/// Extract the typedef name from a list of type specifiers, if present.
fn typedef_name<'a>(specs: &[&'a lang_c::ast::TypeSpecifier]) -> Option<&'a str> {
    specs.iter().find_map(|s| {
        if let lang_c::ast::TypeSpecifier::TypedefName(name) = s {
            Some(name.node.name.as_str())
        } else {
            None
        }
    })
}

/// Map a known C typedef name to its SID pointee type.
fn sid_type_for_typedef(name: &str) -> Option<SidType> {
    match name {
        "size_t" | "uint8_t" | "uint16_t" | "uint32_t" | "uint64_t"
        | "int8_t" | "int16_t" | "int32_t" | "int64_t"
        | "ssize_t" | "ptrdiff_t" | "intptr_t" | "uintptr_t"
        | "off_t" | "pid_t" | "uid_t" | "gid_t" => Some(SidType::Int),
        _ => None,
    }
}


