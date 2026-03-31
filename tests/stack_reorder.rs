use sid::*;
/// Tests for substack-based stack reordering using `($N ...)` templates.
///
/// # How `$N` indexing works
///
/// When a substack template `($1 $2 ...)` is rendered, it consumes N entries
/// from the top of the parent stack.  The consumed entries are indexed from
/// the *bottom* of the consumed slice upward:
///
///   parent stack before: [ ..., A, B ]   ← B is on top
///   consumed_stack:       [ A, B ]        ← index 0 = A, index 1 = B
///   $1 → consumed_stack[0] = A  (the deeper entry)
///   $2 → consumed_stack[1] = B  (the top entry)
///
/// On invocation the substack's program values are reversed onto the program
/// stack, so they execute in the order written — meaning `($1 $2)!` restores
/// the original order (identity) and `($2 $1)!` swaps the top two.
use std::collections::HashMap;

/// Run a sid source snippet and return whatever is left on the data stack.
fn run_snippet(source: &str) -> Vec<DataValue> {
    let parsed = parse_str(source).expect("parse error");
    let mut global_scope = default_scope();
    let comptime_builtins = get_comptime_builtins();
    let after_comptime =
        comptime_pass(parsed.0, &comptime_builtins, &mut global_scope).expect("comptime error");
    let rendered = {
        let mut gs = GlobalState::new(&mut global_scope);
        render_template(
            Template::substack((after_comptime, 0)),
            &mut vec![],
            &HashMap::new(),
            &mut gs,
            &comptime_builtins,
        )
    };
    let instructions: Vec<TemplateValue> = rendered.into_iter().map(TemplateValue::from).collect();
    let mut global_scope_for_run = global_scope;
    let global_state = GlobalState::new(&mut global_scope_for_run);
    let builtins = get_interpret_builtins();
    // Collect results by running interpret and capturing final data stack state.
    let local_scope = HashMap::new();
    let mut exe_state = ExeState {
        program_stack: vec![ProgramValue::Invoke],
        data_stack: instructions,
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
            &builtins,
        );
    }
    exe_state
        .data_stack
        .into_iter()
        .filter_map(|tv| {
            if let TemplateValue::Literal(ProgramValue::Data(v)) = tv {
                Some(v)
            } else {
                None
            }
        })
        .collect()
}

/// `($1 $2)!` — identity: does not change the order of the top two values.
#[test]
fn substack_identity_two_values() {
    // Stack before invoke: [ 10, 20 ] with 20 on top.
    // ($1 $2)! consumes both: $1=10 (deeper), $2=20 (top).
    // Substack body executes 10 then 20 → stack after: [ 10, 20 ] (20 on top).
    let stack = run_snippet("10 20 ($1 $2)!");
    assert_eq!(stack, vec![DataValue::Int(10), DataValue::Int(20)]);
}

/// `($2 $1)!` — swap: exchanges the top two values.
#[test]
fn substack_swap_two_values() {
    // Stack before: [ 10, 20 ] (20 on top).
    // $2=20, $1=10 → substack executes 20 then 10 → stack: [ 20, 10 ] (10 on top).
    let stack = run_snippet("10 20 ($2 $1)!");
    assert_eq!(stack, vec![DataValue::Int(20), DataValue::Int(10)]);
}

/// `($1 $4 $3 $2)!` — the reorder used in test.sid for fgets argument marshaling.
/// Converts stack [ buf, n, file, _ ] into [ buf, file, n, _ ] relative ordering,
/// working over 4 values.
#[test]
fn substack_reorder_four_values() {
    // Push 1 2 3 4 (4 on top).  $1=1 $2=2 $3=3 $4=4.
    // ($1 $4 $3 $2)! → executes 1, 4, 3, 2 → stack: [1, 4, 3, 2] (2 on top).
    let stack = run_snippet("1 2 3 4 ($1 $4 $3 $2)!");
    assert_eq!(
        stack,
        vec![
            DataValue::Int(1),
            DataValue::Int(4),
            DataValue::Int(3),
            DataValue::Int(2),
        ]
    );
}
