use std::collections::HashMap;

use anyhow::{bail, Result};

use crate::{
  InterpretBuiltIn,
  DataValue,
  ProgramValue,
  Template,
  TemplateData,
  TemplateValue,
  render_template,
};

/// Run the comptime pass over a flat sequence of [`TemplateValue`]s.
///
/// The returned `Vec<TemplateValue>` is a modified version of the input:
/// - `@!` sites whose function and argument are both concrete are evaluated
///   and replaced with their results.
/// - Runtime templates are recursed into so nested `@!` sites are also handled.
/// - Comptime templates (`comptime: true`) are rendered eagerly against the
///   current stack and replaced with their concrete `DataValue` result.
pub fn comptime_pass(
  values: Vec<TemplateValue>,
  builtins: &HashMap<&str, &dyn InterpretBuiltIn>,
  scope: &HashMap<String, DataValue>,
) -> Result<Vec<TemplateValue>> {
  let mut stack: Vec<TemplateValue> = Vec::new();

  for tv in values {
    match tv {
      // ── Template: either render eagerly (comptime) or recurse into body ──
      TemplateValue::Literal(ProgramValue::Template(t)) => {
        if t.comptime {
          render_comptime_template(t, &mut stack, scope)?;
        } else {
          let new_data = comptime_pass_template_data(t.data, builtins, scope)?;
          stack.push(TemplateValue::Literal(ProgramValue::Template(
            Template { data: new_data, consumes_stack_entries: t.consumes_stack_entries, comptime: t.comptime }
          )));
        }
      }

      // ── Comptime invoke ───────────────────────────────────────────────────
      TemplateValue::Literal(ProgramValue::ComptimeInvoke) => {
        // Pop function — must be a concrete label.
        let fn_tv = stack.pop().ok_or_else(|| anyhow::anyhow!("@! on empty stack"))?;
        let fn_name = match fn_tv {
          TemplateValue::Literal(ProgramValue::Data(DataValue::Label(l))) => l,
          other => bail!("@! invoked on a non-label value: {:?}", other),
        };

        let builtin = builtins.get(fn_name.as_str())
          .ok_or_else(|| anyhow::anyhow!("Unknown comptime function: '{}'", fn_name))?;

        // Pop argument if the function takes one.
        let arg: Option<DataValue> = if builtin.arg_count() == 1 {
          let arg_tv = stack.pop().ok_or_else(|| anyhow::anyhow!(
            "@! '{}' expected an argument but the stack was empty", fn_name
          ))?;
          match arg_tv {
            TemplateValue::Literal(ProgramValue::Data(v)) => Some(v),
            TemplateValue::Literal(ProgramValue::Template(_)) => bail!(
              "@! '{}': argument is an unrendered template. \
               Did you mean to use a comptime template (@{{...}}, @[...], etc.)?",
              fn_name
            ),
            other => bail!(
              "@! '{}': argument must be a concrete value, got: {:?}",
              fn_name, other
            ),
          }
        } else {
          None
        };

        if let Some(result) = builtin.execute(arg, scope)? {
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
  scope: &HashMap<String, DataValue>,
) -> Result<TemplateData> {
  match data {
    TemplateData::Substack(tvs) =>
      Ok(TemplateData::Substack(comptime_pass(tvs, builtins, scope)?)),
    TemplateData::Script(tvs) =>
      Ok(TemplateData::Script(comptime_pass(tvs, builtins, scope)?)),
    TemplateData::List(tvs) =>
      Ok(TemplateData::List(comptime_pass(tvs, builtins, scope)?)),
    TemplateData::Set(tvs) =>
      Ok(TemplateData::Set(comptime_pass(tvs, builtins, scope)?)),
    TemplateData::Map(pairs) => {
      let mut new_pairs: Vec<(TemplateValue, TemplateValue)> = Vec::new();
      for (k, v) in pairs {
        let mut k_out = comptime_pass(vec![k], builtins, scope)?;
        let mut v_out = comptime_pass(vec![v], builtins, scope)?;
        if k_out.len() != 1 || v_out.len() != 1 {
          bail!("Map key or value produced an unexpected number of elements after the comptime pass");
        }
        new_pairs.push((k_out.remove(0), v_out.remove(0)));
      }
      Ok(TemplateData::Map(new_pairs))
    }
  }
}

/// Render a comptime-marked template eagerly, consuming the required entries
/// from the top of `stack` and pushing the rendered result back.
fn render_comptime_template(
  template: Template,
  stack: &mut Vec<TemplateValue>,
  scope: &HashMap<String, DataValue>,
) -> Result<()> {
  let n = template.consumes_stack_entries;
  if n > stack.len() {
    bail!(
      "Comptime template needs {} parent stack entries but only {} are available",
      n, stack.len()
    );
  }

  // Verify all consumed entries are concrete before mutating the stack.
  for tv in &stack[stack.len() - n..] {
    match tv {
      TemplateValue::Literal(ProgramValue::Data(_)) => {}
      TemplateValue::Literal(ProgramValue::Template(_)) => bail!(
        "Comptime template consumed an unrendered runtime template. \
         Did you mean a comptime template (@{{...}}, @[...], etc.)?"
      ),
      other => bail!(
        "Comptime template consumed a non-concrete value: {:?}", other
      ),
    }
  }

  // Extract parent entries for render_template.
  let stack_len = stack.len();
  let mut parent: Vec<TemplateValue> = stack.drain(stack_len - n..).collect();

  // render_template uses scope as both parent and global scope here.
  let empty_scope = HashMap::new();
  let rendered = render_template(template, &mut parent, &empty_scope, scope);

  for v in rendered {
    stack.push(TemplateValue::from(v));
  }
  Ok(())
}
