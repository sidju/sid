use std::collections::HashMap;

use crate::built_in::BuiltinEntry;
use crate::{
    get_from_scope, DataValue, GlobalState, ProgramValue, Template, TemplateData, TemplateValue,
};

fn resolve_to_program_values(
    source: Vec<TemplateValue>,
    consumed_stack: &mut Vec<Option<DataValue>>,
    parent_scope: &HashMap<String, DataValue>,
    global_scope: &HashMap<String, DataValue>,
    builtins: &HashMap<&'static str, BuiltinEntry>,
) -> Vec<ProgramValue> {
    let builtin_names: std::collections::HashSet<&'static str> = builtins.keys().copied().collect();
    let mut rendered: Vec<ProgramValue> = Vec::new();
    use TemplateValue as TV;
    for entry in source {
        match entry {
            TV::Literal(v) => {
                rendered.push(v);
            }
            TV::ParentLabel(l) => {
                let v = get_from_scope(
                    &l,
                    Some(parent_scope),
                    Some(global_scope),
                    Some(&builtin_names),
                )
                .expect("label resolution failed");
                rendered.push(v.into())
            }
            TV::ParentStackMove(i) => {
                if i == 0 {
                    panic!("Parent stack index 0 is the template!");
                }
                let value = consumed_stack[i - 1]
                    .as_ref()
                    .expect("Stack value missing in template")
                    .clone();
                rendered.push(value.into());
            }
            TV::ComptimeLabel(l) => {
                panic!(
                    "@{} should have been resolved at comptime; render pass should never see it",
                    l
                );
            }
        }
    }
    rendered
}

fn eval_data_seq(
    pvs: Vec<ProgramValue>,
    parent_scope: &HashMap<String, DataValue>,
    global_state: &mut GlobalState,
    builtins: &HashMap<&'static str, BuiltinEntry>,
) -> Vec<DataValue> {
    use crate::invoke::interpret_one;
    let mut data_stack: Vec<TemplateValue> = Vec::new();
    let mut program_stack: Vec<ProgramValue> = pvs.into_iter().rev().collect();
    let mut local_scope = parent_scope.clone();
    let mut scope_stack: Vec<HashMap<String, DataValue>> = Vec::new();
    while !program_stack.is_empty() {
        interpret_one(
            &mut data_stack,
            &mut program_stack,
            &mut local_scope,
            &mut scope_stack,
            global_state,
            builtins,
        );
    }
    data_stack
        .into_iter()
        .map(|tv| match tv {
            TemplateValue::Literal(ProgramValue::Data(v)) => v,
            other => panic!(
                "eval_data_seq: non-data value left on mini-stack: {:?}",
                other
            ),
        })
        .collect()
}

pub fn render_template(
    template: Template,
    parent_stack: &mut Vec<TemplateValue>,
    parent_scope: &HashMap<String, DataValue>,
    global_state: &mut GlobalState,
    builtins: &HashMap<&'static str, BuiltinEntry>,
) -> DataValue {
    if template.consumes_stack_entries > parent_stack.len() {
        panic!("Template consumes more stack entries than there are.");
    }
    let mut consumed_stack: Vec<Option<DataValue>> = parent_stack
        .drain(parent_stack.len() - template.consumes_stack_entries..)
        .map(|tv| match tv {
            TemplateValue::Literal(ProgramValue::Data(v)) => v,
            other => panic!(
                "render_template: parent stack entry is not a concrete DataValue: {:?}",
                other
            ),
        })
        .map(|v| Some(v))
        .collect();

    use TemplateData as TD;
    let rendered_template: DataValue = match template.data {
        TD::Substack(source) => DataValue::Substack {
            body: resolve_to_program_values(
                source,
                &mut consumed_stack,
                parent_scope,
                global_state.scope,
                builtins,
            ),
            args: None,
            ret: None,
        },
        TD::Script(source) => DataValue::Script {
            body: resolve_to_program_values(
                source,
                &mut consumed_stack,
                parent_scope,
                global_state.scope,
                builtins,
            ),
            args: None,
            ret: None,
        },
        TD::List(source) => {
            let pvs = resolve_to_program_values(
                source,
                &mut consumed_stack,
                parent_scope,
                global_state.scope,
                builtins,
            );
            DataValue::List(eval_data_seq(pvs, parent_scope, global_state, builtins))
        }
        TD::Set(source) => {
            let pvs = resolve_to_program_values(
                source,
                &mut consumed_stack,
                parent_scope,
                global_state.scope,
                builtins,
            );
            DataValue::Set(eval_data_seq(pvs, parent_scope, global_state, builtins))
        }
        TD::Map(pairs) => {
            let mut entries: Vec<(DataValue, DataValue)> = Vec::new();
            for (key_tvs, val_tvs) in pairs {
                let key_pvs = resolve_to_program_values(
                    key_tvs,
                    &mut consumed_stack,
                    parent_scope,
                    global_state.scope,
                    builtins,
                );
                let val_pvs = resolve_to_program_values(
                    val_tvs,
                    &mut consumed_stack,
                    parent_scope,
                    global_state.scope,
                    builtins,
                );
                let mut key_vals = eval_data_seq(key_pvs, parent_scope, global_state, builtins);
                let mut val_vals = eval_data_seq(val_pvs, parent_scope, global_state, builtins);
                if key_vals.len() != 1 {
                    panic!(
                        "map key expression must produce exactly 1 value, got {}",
                        key_vals.len()
                    );
                }
                if val_vals.len() != 1 {
                    panic!(
                        "map value expression must produce exactly 1 value, got {}",
                        val_vals.len()
                    );
                }
                entries.push((key_vals.remove(0), val_vals.remove(0)));
            }
            DataValue::Map(entries)
        }
    };

    rendered_template
}
