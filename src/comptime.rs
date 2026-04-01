use std::collections::HashMap;

use anyhow::{bail, Result};

use crate::built_in::BuiltinEntry;
use crate::invoke::ExeState;
use crate::{DataValue, GlobalState, ProgramValue, Template, TemplateData, TemplateValue};

pub fn comptime_pass(
    values: Vec<TemplateValue>,
    builtins: &HashMap<&'static str, BuiltinEntry>,
    scope: &mut HashMap<String, DataValue>,
) -> Result<Vec<TemplateValue>> {
    let mut stack: Vec<TemplateValue> = Vec::new();

    for tv in values {
        match tv {
            TemplateValue::Literal(ProgramValue::Template(t)) => {
                let new_data = comptime_pass_template_data(t.data, builtins, scope)?;
                stack.push(TemplateValue::Literal(ProgramValue::Template(Template {
                    data: new_data,
                    consumes_stack_entries: t.consumes_stack_entries,
                })));
            }

            TemplateValue::Literal(ProgramValue::ComptimeInvoke) => {
                let fn_tv = stack
                    .pop()
                    .ok_or_else(|| anyhow::anyhow!("@! on empty stack"))?;
                let fn_name = match fn_tv {
                    TemplateValue::Literal(ProgramValue::Data(DataValue::Label(l))) => l,
                    other => bail!("@! invoked on a non-label value: {:?}", other),
                };

                let entry = builtins
                    .get(fn_name.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Unknown comptime function: '{}'", fn_name))?;

                let global_state = GlobalState::new(scope);
                let mut exe_state = ExeState {
                    program_stack: Vec::new(),
                    data_stack: stack,
                    local_scope: HashMap::new(),
                    scope_stack: Vec::new(),
                    global_state,
                    builtins,
                };

                let arg_values: Result<Vec<DataValue>> = (0..entry.args.len())
                    .rev()
                    .map(|i| {
                        let tv = &exe_state.data_stack[exe_state.data_stack.len() - 1 - i];
                        match tv {
                            TemplateValue::Literal(ProgramValue::Data(v)) => Ok(v.clone()),
                            other => bail!(
                                "builtin '{}': argument is not concrete: {:?}",
                                fn_name,
                                other
                            ),
                        }
                    })
                    .collect();

                let arg_values = arg_values?;

                for _ in 0..entry.args.len() {
                    exe_state.data_stack.pop();
                }

                let results = (entry.exec)(&mut exe_state, arg_values);

                for result in results {
                    exe_state.data_stack.push(TemplateValue::from(result));
                }

                stack = exe_state.data_stack;
            }

            other => stack.push(other),
        }
    }

    Ok(stack)
}

fn comptime_pass_template_data(
    data: TemplateData,
    builtins: &HashMap<&'static str, BuiltinEntry>,
    scope: &mut HashMap<String, DataValue>,
) -> Result<TemplateData> {
    match data {
        TemplateData::Substack(tvs) => {
            Ok(TemplateData::Substack(comptime_pass(tvs, builtins, scope)?))
        }
        TemplateData::Script(tvs) => Ok(TemplateData::Script(comptime_pass(tvs, builtins, scope)?)),
        TemplateData::List(tvs) => Ok(TemplateData::List(comptime_pass(tvs, builtins, scope)?)),
        TemplateData::Set(tvs) => Ok(TemplateData::Set(comptime_pass(tvs, builtins, scope)?)),
        TemplateData::Map(pairs) => {
            let mut new_pairs: Vec<(Vec<TemplateValue>, Vec<TemplateValue>)> = Vec::new();
            for (k_tvs, v_tvs) in pairs {
                let k_out = comptime_pass(k_tvs, builtins, scope)?;
                let v_out = comptime_pass(v_tvs, builtins, scope)?;
                new_pairs.push((k_out, v_out));
            }
            Ok(TemplateData::Map(new_pairs))
        }
    }
}
