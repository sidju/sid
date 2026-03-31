use std::collections::HashMap;

use super::{
    call_c_function, call_cfuncsig, get_from_scope, render_template, resolve_if_label, DataValue,
    GlobalState, InterpretBuiltIn, ProgramValue, SidType, TemplateValue,
};

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
    builtins: &HashMap<&str, &dyn InterpretBuiltIn>,
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
            // If it's a label, try resolving it before failing.
            if let DataValue::Label(_) = actual {
                let resolved = resolve_if_label(
                    actual.clone(),
                    Some(local_scope),
                    Some(global_scope),
                    Some(builtins),
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
}

pub fn invoke<'a, 'b>(
    data_stack: &mut Vec<TemplateValue>,
    program_stack: &mut Vec<ProgramValue>,
    local_scope: &mut HashMap<String, DataValue>,
    global_state: &mut GlobalState<'a>,
    builtins: &HashMap<&'b str, &'b dyn InterpretBuiltIn>,
) {
    // Resolve labels via scope, falling back to built-ins at lowest priority.
    let value = match data_stack.pop() {
        Some(TemplateValue::Literal(ProgramValue::Data(DataValue::Label(l)))) => get_from_scope(
            &l,
            Some(local_scope),
            Some(global_state.scope),
            Some(builtins),
        )
        .expect("label resolution failed"),
        Some(TemplateValue::Literal(ProgramValue::Data(v))) => v,
        Some(other) => panic!("Invoked on non-data stack entry: {:?}", other),
        None => panic!("Invoked on empty data_stack!"),
    };
    match value {
        // Invoking a substack: check arg types (if declared), schedule ret check
        // (if declared), then push the body onto the program stack.
        // Both args and ret are stored top-first (index 0 = top of stack).
        //
        // Every substack gets a fresh local scope (PushScope / PopScope sentinels).
        // When `args` are declared a `StackBlock` is inserted below the top N items
        // on the data stack so the body cannot accidentally read the caller's stack.
        // The `TypeCheck` sentinel (with `block_placed: true`) fires after the body
        // and PopScope to remove the block and optionally verify return types.
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
                // Struct form: if N > 1 and the top of stack is a Map with matching keys,
                // pop it and push individual values in top-first order so the existing
                // check_type_contract + PushScope machinery handles them normally.
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
                            // Push in reverse of names order so top of stack = names[0] (top-first convention).
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
                                    Some(builtins),
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
                    builtins,
                );
                // Insert StackBlock below the args; args will be consumed by PushScope.
                let n = arg_fields.len();
                let insert_pos = data_stack.len() - n;
                data_stack.insert(insert_pos, TemplateValue::from(DataValue::StackBlock));
            }
            // Schedule cleanup / ret check (fires last, after PopScope).
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

        // Invoking a built-in via the InterpretBuiltIn trait:
        // arg_count/return_count determine stack interaction.
        DataValue::BuiltIn(function) => {
            let builtin = builtins[&function[..]];
            for result in builtin
                .execute(
                    data_stack,
                    global_state,
                    program_stack,
                    local_scope,
                    builtins,
                )
                .unwrap_or_else(|e| panic!("BuiltIn '{}' returned error: {}", function, e))
            {
                data_stack.push(TemplateValue::from(result));
            }
        }
        // Invoking a linked CFuncSig: look up the symbol by name at call time.
        DataValue::CFuncSig(sig) => {
            let ctx = format!("CFuncSig '{}'", sig.name);
            let resolve = |v: DataValue| match v {
                DataValue::Label(ref l) => get_from_scope(
                    l,
                    Some(local_scope),
                    Some(global_state.scope),
                    Some(builtins),
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
        // Invoking a dynamically-loaded C function via libffi.
        DataValue::CFunction(f) => {
            let ctx = format!("CFunction '{}'", f.name);
            let resolve = |v: DataValue| match v {
                DataValue::Label(ref l) => get_from_scope(
                    l,
                    Some(local_scope),
                    Some(global_state.scope),
                    Some(builtins),
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

pub fn interpret<'a, 'b>(
    program: Vec<ProgramValue>,
    data_stack: Vec<TemplateValue>,
    global_state: GlobalState<'a>,
    builtins: &HashMap<&'b str, &'b dyn InterpretBuiltIn>,
) {
    let local_scope = HashMap::new();
    let mut exe_state = ExeState {
        program_stack: program,
        data_stack,
        local_scope,
        scope_stack: Vec::new(),
        global_state,
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

pub fn interpret_one<'a, 'b>(
    data_stack: &mut Vec<TemplateValue>,
    program_stack: &mut Vec<ProgramValue>,
    local_scope: &mut HashMap<String, DataValue>,
    scope_stack: &mut Vec<HashMap<String, DataValue>>,
    global_state: &mut GlobalState<'a>,
    builtins: &HashMap<&'b str, &'b dyn InterpretBuiltIn>,
) {
    use ProgramValue as PV;
    let operation = program_stack.pop().unwrap();
    match operation {
        PV::Data(v) => {
            data_stack.push(TemplateValue::Literal(PV::Data(v)));
        }
        PV::Template(t) => {
            let rendered = render_template(t, data_stack, local_scope, global_state, builtins);
            data_stack.extend(rendered.into_iter().map(TemplateValue::from));
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
                // Push in reverse execution order: CondLoop (last) → cond+Invoke → body+Invoke (first).
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
            // Initial condition of a while_do just ran. Pop its Bool and, if true,
            // capture expected_len from the current stack and schedule subsequent iters.
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
                // Push in reverse execution order: CondLoop (last) → cond+Invoke → body+Invoke (first).
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
                // Find the StackBlock inserted at invocation time.
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
                        builtins,
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
                    builtins,
                );
            }
        }
        PV::PushScope { names } => {
            let old_scope = std::mem::replace(local_scope, HashMap::new());
            scope_stack.push(old_scope);
            // Consume args from the top of the data stack (top-first order matches names[0]).
            // Labels are already resolved in-place by check_type_contract if needed.
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
