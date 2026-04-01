use std::collections::HashMap;

use anyhow::{bail, Result};

use crate::built_in::BuiltinEntry;
use crate::invoke::ExeState;
use crate::render::render_template;
use crate::{DataValue, GlobalState, ProgramValue, Template, TemplateData, TemplateValue};

fn is_template_concrete(data: &TemplateData) -> bool {
    match data {
        TemplateData::Substack(tvs)
        | TemplateData::Script(tvs)
        | TemplateData::List(tvs)
        | TemplateData::Set(tvs) => tvs.iter().all(template_value_is_concrete),
        TemplateData::Map(pairs) => pairs.iter().all(|(k, v)| {
            k.iter().all(template_value_is_concrete) && v.iter().all(template_value_is_concrete)
        }),
    }
}

fn template_value_is_concrete(tv: &TemplateValue) -> bool {
    match tv {
        TemplateValue::ParentStackMove(_) | TemplateValue::ParentLabel(_) => false,
        TemplateValue::Literal(_) => true,
        TemplateValue::ComptimeLabel(_) => false,
    }
}

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

                // Pop arguments from stack before building exe_state to avoid borrow conflicts.
                let args_start = stack.len() - entry.args.len();
                let arg_tvs: Vec<TemplateValue> = stack.drain(args_start..).collect();

                // Render concrete templates; pass concrete DataValues through.
                let arg_values: Result<Vec<DataValue>> = arg_tvs
                    .iter()
                    .map(|tv| match tv {
                        TemplateValue::Literal(ProgramValue::Data(v)) => Ok(v.clone()),
                        TemplateValue::Literal(ProgramValue::Template(t)) => {
                            if is_template_concrete(&t.data) {
                                let mut parent_stack: Vec<TemplateValue> = Vec::new();
                                let mut empty_scope = HashMap::new();
                                let mut gs = GlobalState::new(&mut empty_scope);
                                let template = Template {
                                    data: t.data.clone(),
                                    consumes_stack_entries: t.consumes_stack_entries,
                                };
                                let dummy_scope = HashMap::new();
                                Ok(render_template(
                                    template,
                                    &mut parent_stack,
                                    &dummy_scope,
                                    &mut gs,
                                    builtins,
                                ))
                            } else {
                                bail!(
                                    "builtin '{}': argument template is not concrete: {:?}",
                                    fn_name,
                                    tv
                                )
                            }
                        }
                        other => bail!(
                            "builtin '{}': argument is not concrete: {:?}",
                            fn_name,
                            other
                        ),
                    })
                    .collect();

                let arg_values = arg_values?;

                let mut exe_state = ExeState {
                    program_stack: Vec::new(),
                    data_stack: stack,
                    local_scope: HashMap::new(),
                    scope_stack: Vec::new(),
                    global_state: GlobalState::new(scope),
                    builtins,
                };

                let results = (entry.exec)(&mut exe_state, arg_values);

                for result in results {
                    exe_state.data_stack.push(TemplateValue::from(result));
                }

                stack = exe_state.data_stack;
            }

            TemplateValue::ComptimeLabel(label) => {
                let value = crate::get_from_scope(&label, None, Some(scope), None)
                    .map_err(|e| anyhow::anyhow!("@{}: {}", label, e))?;
                stack.push(TemplateValue::Literal(ProgramValue::Data(value)));
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
