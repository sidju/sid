use std::collections::HashMap;

use sid::*;

use clap::Parser;

struct Program {
  instructions: Vec<DataValue>,
  global_scope: HashMap<String, RealValue>,
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

fn compile(source: & str) -> Program {
  // Parse that String into Vec<TemplateValue>
  let parsed = parse_str(source).expect("parse error");
  // Create the global scope, with built-in functions and constants
  // (Implementations of this don't yet exist)
  let global_scope = HashMap::new();
  // Render that Vec<TemplateValue> as a Substack
  let rendered = render_template(
    Template::substack(parsed),
    &mut Vec::new(), // Current stack, not applicable
    &mut HashMap::new(), // Current local scope, not applicable
    &global_scope,
  );
  return Program{
    instructions: rendered,
    global_scope,
  };
}

fn run(source: &str) {
  let program = compile(source);
  // The rendering output is the whole data stack initially
  // And an invoke the whole program
  let program_stack = vec![ProgramValue::Invoke];
  let data_stack = program.instructions;
  let built_in_functions = get_built_in_functions();
  interpret(
    program_stack,
    data_stack,
    program.global_scope,
    &built_in_functions,
  );
}
