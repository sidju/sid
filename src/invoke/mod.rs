use std::collections::HashMap;

use super::{
    call_c_function, call_cfuncsig, get_from_scope, render_template, resolve_if_label, DataValue,
    GlobalState, ProgramValue, SidType, TemplateValue,
};
use crate::built_in::BuiltinEntry;

/// Collect arguments for a callable from the data stack, supporting two
/// calling conventions for N > 1 fixed params:
///
/// - **Stack form**: N items on the stack (top = last declared param).
/// - **Struct form**: a single `DataValue::Map` whose label-keys exactly match
///   `param_names`; values are extracted in declaration order.
///   Only attempted when `param_names` is non-empty and N > 1.
///
/// For variadic callables (`variadic = true`):
/// - Stack form: top item is a `List` of variadic args; below it are the N
///   fixed params individually.
/// - Struct form: top item is a Map with keys matching fixed params plus `"..."`;
///   the `"..."` value must be a List of variadic args.
///
/// Each collected value is passed through `resolve` (label resolution).
/// Returns `None` for 0-param functions.
fn collect_args(
    data_stack: &mut Vec<TemplateValue>,
    param_names: &[String],
    n: usize,
    variadic: bool,
    resolve: &dyn Fn(DataValue) -> DataValue,
    context: &str,
) -> Option<DataValue> {
    let pop_one = |stack: &mut Vec<TemplateValue>, ctx: &str| -> DataValue {
        match stack.pop() {
            Some(TemplateValue::Literal(ProgramValue::Data(v))) => v,
            Some(other) => panic!("{}: argument is not a concrete value: {:?}", ctx, other),
            None => panic!("{}: expected argument but stack was empty", ctx),
        }
    };

    if variadic {
        // Struct form for variadic: single Map with fixed keys + "..." key.
        if !param_names.is_empty() {
            if let Some(TemplateValue::Literal(ProgramValue::Data(DataValue::Map(_)))) =
                data_stack.last()
            {
                if let Some(TemplateValue::Literal(ProgramValue::Data(DataValue::Map(entries)))) =
                    data_stack.last().cloned()
                {
                    let map_keys: std::collections::HashSet<&str> = entries
                        .iter()
                        .filter_map(|(k, _)| {
                            if let DataValue::Label(l) = k {
                                Some(l.as_str())
                            } else {
                                None
                            }
                        })
                        .collect();
                    let all_fixed_match = param_names[..n]
                        .iter()
                        .all(|p| map_keys.contains(p.as_str()));
                    let has_variadic_key = map_keys.contains("...");
                    if all_fixed_match && has_variadic_key && map_keys.len() == n + 1 {
                        data_stack.pop(); // consume the map
                        let mut items: Vec<DataValue> = param_names[..n]
                            .iter()
                            .map(|name| {
                                let v = entries
                                    .iter()
                                    .find(|(k, _)| matches!(k, DataValue::Label(l) if l == name))
                                    .map(|(_, v)| v.clone())
                                    .unwrap_or_else(|| unreachable!());
                                resolve(v)
                            })
                            .collect();
                        let variadic_list = entries
                            .iter()
                            .find(|(k, _)| matches!(k, DataValue::Label(l) if l == "..."))
                            .map(|(_, v)| v.clone())
                            .unwrap_or(DataValue::List(vec![]));
                        let variadic_items = match variadic_list {
                            DataValue::List(vs) => vs.into_iter().map(resolve).collect::<Vec<_>>(),
                            other => {
                                panic!("{}: '...' key must be a List, got {:?}", context, other)
                            }
                        };
                        items.extend(variadic_items);
                        return Some(DataValue::List(items));
                    }
                }
            }
        }
        // Stack form for variadic: top = List of variadic args, below = N fixed params.
        let variadic_val = pop_one(data_stack, context);
        let variadic_items = match resolve(variadic_val) {
            DataValue::List(vs) => vs.into_iter().map(resolve).collect::<Vec<_>>(),
            other => panic!(
                "{}: expected List of variadic args on top of stack, got {:?}",
                context, other
            ),
        };
        let mut fixed: Vec<DataValue> = (0..n)
            .map(|_| resolve(pop_one(data_stack, context)))
            .collect();
        fixed.reverse();
        fixed.extend(variadic_items);
        return Some(DataValue::List(fixed));
    }

    match n {
        0 => None,
        1 => Some(resolve(pop_one(data_stack, context))),
        _ => {
            // Struct form: single Map with keys matching all param names.
            if !param_names.is_empty() {
                if let Some(TemplateValue::Literal(ProgramValue::Data(DataValue::Map(_)))) =
                    data_stack.last()
                {
                    if let Some(TemplateValue::Literal(ProgramValue::Data(DataValue::Map(
                        ref entries,
                    )))) = data_stack.last().cloned()
                    {
                        let map_keys: std::collections::HashSet<&str> = entries
                            .iter()
                            .filter_map(|(k, _)| {
                                if let DataValue::Label(l) = k {
                                    Some(l.as_str())
                                } else {
                                    None
                                }
                            })
                            .collect();
                        if map_keys.len() == n
                            && param_names.iter().all(|p| map_keys.contains(p.as_str()))
                        {
                            data_stack.pop();
                            let items: Vec<DataValue> =
                                param_names
                                    .iter()
                                    .map(|name| {
                                        let v = entries.iter()
                  .find(|(k, _)| matches!(k, DataValue::Label(l) if l == name))
                  .map(|(_, v)| v.clone())
                  .unwrap_or_else(|| unreachable!());
                                        resolve(v)
                                    })
                                    .collect();
                            return Some(DataValue::List(items));
                        }
                    }
                }
            }
            // Stack form: pop N items, reverse to get declaration order.
            let mut items: Vec<DataValue> = (0..n)
                .map(|_| resolve(pop_one(data_stack, context)))
                .collect();
            items.reverse();
            Some(DataValue::List(items))
        }
    }
}

/// Check the top of `data_stack` against a type slice, resolving any labels
/// via scope before matching.
///
/// `types[0]` = top of stack, `types[N-1]` = deepest checked item.
/// Panics with a detailed message on the first mismatch, or if the stack is
/// too shallow. `label` ("args"/"ret") and `context` (callable description)
/// are included in any panic message.
///
/// When a label is found that doesn't match the expected type, it is resolved
/// from scope and the resolved value replaces it in `data_stack` in-place
/// before the check proceeds.
pub(crate) fn check_type_contract(
    data_stack: &mut [TemplateValue],
    types: &[SidType],
    label: &str,
    context: &str,
    local_scope: &HashMap<String, DataValue>,
    global_scope: &HashMap<String, DataValue>,
    builtin_names: &std::collections::HashSet<&'static str>,
) {
    if data_stack.len() < types.len() {
        panic!(
            "{} {} check failed: expected {} items on stack, only {} available",
            context,
            label,
            types.len(),
            data_stack.len()
        );
    }
    let stack_len = data_stack.len();
    for (i, expected) in types.iter().enumerate() {
        let stack_idx = stack_len - 1 - i;
        let tv = &data_stack[stack_idx];
        let actual = match tv {
            TemplateValue::Literal(ProgramValue::Data(v)) => v,
            other => panic!(
        "{} {} check failed: position {} (0=top): expected {:?}, got non-concrete value {:?}",
        context, label, i, expected, other
      ),
        };
        if !expected.matches(actual) {
            if let DataValue::Label(_) = actual {
                let resolved = resolve_if_label(
                    actual.clone(),
                    Some(local_scope),
                    Some(global_scope),
                    Some(builtin_names),
                );
                if expected.matches(&resolved) {
                    data_stack[stack_idx] = TemplateValue::from(resolved);
                    continue;
                }
                panic!(
          "{} {} check failed: position {} (0=top): expected {:?}, label resolved to {:?}",
          context, label, i, expected, &data_stack[stack_idx]
        );
            }
            panic!(
                "{} {} check failed: position {} (0=top): expected {:?}, got {:?}",
                context, label, i, expected, actual
            );
        }
    }
}

pub struct ExeState<'a> {
    pub program_stack: Vec<ProgramValue>,
    pub data_stack: Vec<TemplateValue>,
    pub local_scope: HashMap<String, DataValue>,
    /// Scope stack used by `PushScope`/`PopScope` sentinels to isolate substack
    /// local bindings.  Each `PushScope` pushes the current `local_scope` here
    /// and installs a fresh empty one; `PopScope` restores it.
    pub scope_stack: Vec<HashMap<String, DataValue>>,
    pub global_state: GlobalState<'a>,
    pub builtins: &'a HashMap<&'static str, BuiltinEntry>,
}

pub fn invoke<'a>(
    data_stack: &mut Vec<TemplateValue>,
    program_stack: &mut Vec<ProgramValue>,
    local_scope: &mut HashMap<String, DataValue>,
    global_state: &mut GlobalState<'a>,
    builtins: &HashMap<&'static str, BuiltinEntry>,
) {
    let builtin_names: std::collections::HashSet<&'static str> = builtins.keys().copied().collect();
    let value = match data_stack.pop() {
        Some(TemplateValue::Literal(ProgramValue::Data(DataValue::Label(l)))) => get_from_scope(
            &l,
            Some(local_scope),
            Some(global_state.scope),
            Some(&builtin_names),
        )
        .expect("label resolution failed"),
        Some(TemplateValue::Literal(ProgramValue::Data(v))) => v,
        Some(other) => panic!("Invoked on non-data stack entry: {:?}", other),
        None => panic!("Invoked on empty data_stack!"),
    };
    match value {
        DataValue::Substack {
            body: mut s,
            args,
            ret,
        } => {
            s.reverse();
            let block_placed = args.is_some();
            let names: Vec<String> = args
                .as_ref()
                .map(|a| a.iter().map(|(n, _)| n.clone()).collect())
                .unwrap_or_default();
            if let Some(ref arg_fields) = args {
                let n = arg_fields.len();
                if n > 1 {
                    if let Some(TemplateValue::Literal(ProgramValue::Data(DataValue::Map(
                        ref entries,
                    )))) = data_stack.last().cloned()
                    {
                        let map_keys: std::collections::HashSet<&str> = entries
                            .iter()
                            .filter_map(|(k, _)| {
                                if let DataValue::Label(l) = k {
                                    Some(l.as_str())
                                } else {
                                    None
                                }
                            })
                            .collect();
                        if map_keys.len() == n
                            && names.iter().all(|p| map_keys.contains(p.as_str()))
                        {
                            data_stack.pop();
                            for name in names.iter().rev() {
                                let v = entries
                                    .iter()
                                    .find(|(k, _)| matches!(k, DataValue::Label(l) if l == name))
                                    .map(|(_, v)| v.clone())
                                    .unwrap_or_else(|| unreachable!());
                                let v = resolve_if_label(
                                    v,
                                    Some(local_scope),
                                    Some(global_state.scope),
                                    Some(&builtin_names),
                                );
                                data_stack.push(TemplateValue::from(v));
                            }
                        }
                    }
                }
                let arg_types: Vec<SidType> = arg_fields.iter().map(|(_, t)| t.clone()).collect();
                check_type_contract(
                    data_stack,
                    &arg_types,
                    "args",
                    "substack",
                    local_scope,
                    global_state.scope,
                    &builtin_names,
                );
                let n = arg_fields.len();
                let insert_pos = data_stack.len() - n;
                data_stack.insert(insert_pos, TemplateValue::from(DataValue::StackBlock));
            }
            match (&args, &ret) {
                (None, None) => {}
                _ => program_stack.push(ProgramValue::TypeCheck {
                    types: ret,
                    context: "substack ret".to_owned(),
                    block_placed,
                }),
            }
            program_stack.push(ProgramValue::PopScope);
            program_stack.append(&mut s);
            program_stack.push(ProgramValue::PushScope { names });
        }

        DataValue::BuiltIn(name) => {
            let entry = &builtins[name.as_str()];
            let mut exe_state = ExeState {
                program_stack: std::mem::take(program_stack),
                data_stack: std::mem::take(data_stack),
                local_scope: std::mem::take(local_scope),
                scope_stack: Vec::new(),
                global_state: GlobalState::new(global_state.scope),
                builtins,
            };
            exe_state.global_state.libraries = std::mem::take(&mut global_state.libraries);

            let arg_values: Vec<DataValue> = entry
                .args
                .iter()
                .enumerate()
                .rev()
                .map(|(i, expected_type)| {
                    let stack_idx = exe_state.data_stack.len() - 1 - i;
                    let tv = exe_state.data_stack[stack_idx].clone();
                    let v = match tv {
                        TemplateValue::Literal(ProgramValue::Data(v)) => v,
                        other => {
                            panic!("builtin '{}': argument is not concrete: {:?}", name, other)
                        }
                    };
                    let should_keep_label = matches!(expected_type, SidType::Label);
                    if expected_type.matches(&v)
                        && (should_keep_label || !matches!(v, DataValue::Label(_)))
                    {
                        v
                    } else if let DataValue::Label(ref l) = v {
                        let resolved = get_from_scope(
                            l,
                            Some(&exe_state.local_scope),
                            Some(exe_state.global_state.scope),
                            Some(&builtin_names),
                        )
                        .unwrap_or_else(|_| v.clone());
                        if expected_type.matches(&resolved) {
                            resolved
                        } else {
                            panic!(
                                "builtin '{}': arg {} expected {:?}, label '{}' resolved to {:?}",
                                name, i, expected_type, l, resolved
                            );
                        }
                    } else if expected_type.matches(&v) {
                        v
                    } else {
                        panic!(
                            "builtin '{}': arg {} expected {:?}, got {:?}",
                            name, i, expected_type, v
                        );
                    }
                })
                .collect();

            for _ in 0..entry.args.len() {
                exe_state.data_stack.pop();
            }

            let results = (entry.exec)(&mut exe_state, arg_values);

            for result in results {
                exe_state.data_stack.push(TemplateValue::from(result));
            }

            *program_stack = exe_state.program_stack;
            *data_stack = exe_state.data_stack;
            *local_scope = exe_state.local_scope;
            global_state.libraries = exe_state.global_state.libraries;
        }
        DataValue::CFuncSig(sig) => {
            let ctx = format!("CFuncSig '{}'", sig.name);
            let resolve = |v: DataValue| match v {
                DataValue::Label(ref l) => get_from_scope(
                    l,
                    Some(local_scope),
                    Some(global_state.scope),
                    Some(&builtin_names),
                )
                .unwrap_or_else(|e| panic!("CFuncSig '{}': {}", sig.name, e)),
                other => other,
            };
            let arg = collect_args(
                data_stack,
                &sig.param_names,
                sig.params.len(),
                sig.variadic,
                &resolve,
                &ctx,
            );
            if let Some(result) = call_cfuncsig(&sig, arg, &global_state.libraries)
                .unwrap_or_else(|e| panic!("CFuncSig '{}' call error: {}", sig.name, e))
            {
                data_stack.push(TemplateValue::from(result));
            }
        }
        DataValue::CFunction(f) => {
            let ctx = format!("CFunction '{}'", f.name);
            let resolve = |v: DataValue| match v {
                DataValue::Label(ref l) => get_from_scope(
                    l,
                    Some(local_scope),
                    Some(global_state.scope),
                    Some(&builtin_names),
                )
                .unwrap_or_else(|e| panic!("CFunction '{}': {}", f.name, e)),
                other => other,
            };
            let arg = collect_args(
                data_stack,
                &f.sig.param_names,
                f.sig.params.len(),
                f.sig.variadic,
                &resolve,
                &ctx,
            );
            if let Some(result) = call_c_function(&f, arg)
                .unwrap_or_else(|e| panic!("CFunction '{}' returned error: {}", f.name, e))
            {
                data_stack.push(TemplateValue::from(result));
            }
        }
        _ => panic!("Invalid object invoked."),
    }
}

pub fn interpret<'a>(
    program: Vec<ProgramValue>,
    data_stack: Vec<TemplateValue>,
    global_state: GlobalState<'a>,
    builtins: &HashMap<&'static str, BuiltinEntry>,
) {
    let local_scope = HashMap::new();
    let mut exe_state = ExeState {
        program_stack: program,
        data_stack,
        local_scope,
        scope_stack: Vec::new(),
        global_state,
        builtins,
    };
    while !exe_state.program_stack.is_empty() {
        interpret_one(
            &mut exe_state.data_stack,
            &mut exe_state.program_stack,
            &mut exe_state.local_scope,
            &mut exe_state.scope_stack,
            &mut exe_state.global_state,
            builtins,
        )
    }
}

pub fn interpret_one<'a>(
    data_stack: &mut Vec<TemplateValue>,
    program_stack: &mut Vec<ProgramValue>,
    local_scope: &mut HashMap<String, DataValue>,
    scope_stack: &mut Vec<HashMap<String, DataValue>>,
    global_state: &mut GlobalState<'a>,
    builtins: &HashMap<&'static str, BuiltinEntry>,
) {
    use ProgramValue as PV;
    let operation = program_stack.pop().unwrap();
    let builtin_names: std::collections::HashSet<&'static str> = builtins.keys().copied().collect();
    match operation {
        PV::Data(v) => {
            data_stack.push(TemplateValue::Literal(PV::Data(v)));
        }
        PV::Template(t) => {
            let rendered = render_template(t, data_stack, local_scope, global_state, builtins);
            data_stack.push(TemplateValue::from(rendered));
        }
        PV::Invoke | PV::ComptimeInvoke => {
            invoke(
                data_stack,
                program_stack,
                local_scope,
                global_state,
                builtins,
            );
        }
        PV::StackSizeAssert {
            expected_len,
            message,
        } => {
            if data_stack.len() != expected_len {
                panic!(
                    "{} (expected stack size {}, got {})",
                    message,
                    expected_len,
                    data_stack.len()
                );
            }
        }
        PV::CondLoop {
            cond,
            body,
            expected_len,
        } => {
            if data_stack.len() != expected_len + 1 {
                panic!(
          "loop condition must leave exactly one Bool on top (expected stack size {}, got {})",
          expected_len + 1, data_stack.len()
        );
            }
            let bool_val = match data_stack.pop() {
                Some(TemplateValue::Literal(ProgramValue::Data(DataValue::Bool(b)))) => b,
                Some(other) => panic!(
                    "loop condition must leave a Bool on top of the stack, got {:?}",
                    other
                ),
                None => unreachable!(),
            };
            if bool_val {
                program_stack.push(PV::CondLoop {
                    cond: cond.clone(),
                    body: body.clone(),
                    expected_len,
                });
                program_stack.push(PV::Invoke);
                program_stack.push(PV::Data(cond));
                program_stack.push(PV::Invoke);
                program_stack.push(PV::Data(body));
            }
        }
        PV::CondLoopStart { cond, body } => {
            let bool_val = match data_stack.pop() {
                Some(TemplateValue::Literal(ProgramValue::Data(DataValue::Bool(b)))) => b,
                Some(other) => panic!(
                    "while_do condition must leave a Bool on top of the stack, got {:?}",
                    other
                ),
                None => {
                    panic!("while_do condition must leave a Bool on top of the stack (stack empty)")
                }
            };
            if bool_val {
                let expected_len = data_stack.len();
                program_stack.push(PV::CondLoop {
                    cond: cond.clone(),
                    body: body.clone(),
                    expected_len,
                });
                program_stack.push(PV::Invoke);
                program_stack.push(PV::Data(cond));
                program_stack.push(PV::Invoke);
                program_stack.push(PV::Data(body));
            }
        }
        PV::TypeCheck {
            types,
            context,
            block_placed,
        } => {
            if block_placed {
                let block_pos = data_stack
                    .iter()
                    .rposition(|tv| {
                        matches!(
                            tv,
                            TemplateValue::Literal(ProgramValue::Data(DataValue::StackBlock))
                        )
                    })
                    .unwrap_or_else(|| {
                        panic!("TypeCheck ({}): no StackBlock found on data stack", context)
                    });
                if let Some(ret_types) = types {
                    let results = &data_stack[block_pos + 1..];
                    if results.len() != ret_types.len() {
                        panic!(
                            "TypeCheck ({}): expected {} return value(s) above StackBlock, got {}",
                            context,
                            ret_types.len(),
                            results.len()
                        );
                    }
                    let results_mut = &mut data_stack[block_pos + 1..];
                    check_type_contract(
                        results_mut,
                        &ret_types,
                        "ret",
                        &context,
                        local_scope,
                        global_state.scope,
                        &builtin_names,
                    );
                }
                data_stack.remove(block_pos);
            } else if let Some(ret_types) = types {
                check_type_contract(
                    data_stack,
                    &ret_types,
                    "ret",
                    &context,
                    local_scope,
                    global_state.scope,
                    &builtin_names,
                );
            }
        }
        PV::PushScope { names } => {
            let old_scope = std::mem::replace(local_scope, HashMap::new());
            scope_stack.push(old_scope);
            for name in names.into_iter() {
                let value = match data_stack.pop() {
                    Some(TemplateValue::Literal(ProgramValue::Data(v))) => v,
                    Some(other) => panic!(
                        "PushScope: arg '{}' is not a concrete value: {:?}",
                        name, other
                    ),
                    None => panic!(
                        "PushScope: expected arg '{}' but data stack was empty",
                        name
                    ),
                };
                local_scope.insert(name, value);
            }
        }
        PV::PopScope => {
            *local_scope = scope_stack
                .pop()
                .expect("PopScope with no matching PushScope");
        }
    }
}
