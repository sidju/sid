use std::collections::HashMap;

use anyhow::{bail, Result};

use crate::{
    DataValue, GlobalState, InterpretBuiltIn, ProgramValue, Template, TemplateData, TemplateValue,
};

/// Run the comptime pass over a flat sequence of [`TemplateValue`]s.
///
/// The returned `Vec<TemplateValue>` is a modified version of the input:
/// - `@!` sites whose function and argument are both concrete are evaluated
///   and replaced with their results.
/// - Runtime templates are recursed into so nested `@!` sites are also handled.
pub fn comptime_pass(
    values: Vec<TemplateValue>,
    builtins: &HashMap<&str, &dyn InterpretBuiltIn>,
    scope: &mut HashMap<String, DataValue>,
) -> Result<Vec<TemplateValue>> {
    let mut stack: Vec<TemplateValue> = Vec::new();

    for tv in values {
        match tv {
            // ── Template: recurse into body to handle nested @! sites ────────────
            TemplateValue::Literal(ProgramValue::Template(t)) => {
                let new_data = comptime_pass_template_data(t.data, builtins, scope)?;
                stack.push(TemplateValue::Literal(ProgramValue::Template(Template {
                    data: new_data,
                    consumes_stack_entries: t.consumes_stack_entries,
                })));
            }

            // ── Comptime invoke ───────────────────────────────────────────────────
            TemplateValue::Literal(ProgramValue::ComptimeInvoke) => {
                // Pop function — must be a concrete label.
                let fn_tv = stack
                    .pop()
                    .ok_or_else(|| anyhow::anyhow!("@! on empty stack"))?;
                let fn_name = match fn_tv {
                    TemplateValue::Literal(ProgramValue::Data(DataValue::Label(l))) => l,
                    other => bail!("@! invoked on a non-label value: {:?}", other),
                };

                let builtin = builtins
                    .get(fn_name.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Unknown comptime function: '{}'", fn_name))?;

                let mut gs = GlobalState::new(scope);
                for result in builtin.execute(
                    &mut stack,
                    &mut gs,
                    &mut vec![],
                    &mut HashMap::new(),
                    builtins,
                )? {
                    stack.push(TemplateValue::from(result));
                }
            }

            // ── Everything else: pass through ─────────────────────────────────────
            other => stack.push(other),
        }
    }

    Ok(stack)
}

/// Recursively apply the comptime pass to all inner [`TemplateData`] bodies.
fn comptime_pass_template_data(
    data: TemplateData,
    builtins: &HashMap<&str, &dyn InterpretBuiltIn>,
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
