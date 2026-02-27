use std::collections::HashMap;

use sid::*;

use clap::Parser;

struct Program {
  instructions: Vec<TemplateValue>,
  global_scope: HashMap<String, DataValue>,
}

#[derive(Parser)]
#[command(version)]
struct CliArgs {
  /// Path to file to execute code from or `-` for stdin
  file: String,
}

fn main() {
  let cli = CliArgs::parse();
  run_file(&cli.file.clone());
}

fn run_file(path: &str) {
  // Create a String from the file
  let file_content = std::fs::read_to_string(path)
    .expect("Failed to read file");
  run(&file_content);
}

fn compile(source: &str) -> Program {
  let parsed = parse_str(source).expect("parse error");
  let global_scope = HashMap::new();
  let comptime_builtins = get_comptime_builtins();
  let after_comptime = comptime_pass(parsed.0, &comptime_builtins, &global_scope)
    .expect("comptime error");
  // Wrap the post-comptime sequence as a substack and render it to get the
  // initial data stack (TemplateValue entries ready for the interpreter).
  let rendered = render_template(
    Template::substack((after_comptime, 0)),
    &mut Vec::new(),
    &HashMap::new(),
    &global_scope,
  );
  // render_template returns Vec<DataValue>; lift them into TemplateValue.
  let instructions = rendered.into_iter().map(TemplateValue::from).collect();
  Program { instructions, global_scope }
}

fn run(source: &str) {
  let program = compile(source);
  let program_stack = vec![ProgramValue::Invoke];
  let runtime_builtins = get_interpret_builtins();
  interpret(
    program_stack,
    program.instructions,
    program.global_scope,
    &runtime_builtins,
  );
}
