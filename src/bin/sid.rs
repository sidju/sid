use std::collections::HashMap;

use sid::*;

use clap::Parser;

#[derive(Parser)]
#[command(version)]
struct CliArgs {
  /// Path to file to execute code from or `-` for stdin
  file: String,
}
fn main() {
  let cli = CliArgs::parse();
  // Create a String from the file
  let file_content = std::fs::read_to_string(cli.file.clone())
    .expect("Failed to read file");

  // Parse that String into Vec<TemplateValue>
  let parsed = parse_str(&file_content);
  // Create the global scope, with built-in functions and constants
  // (Implementations of this don't yet exist)
  let mut global_scope = HashMap::new();
  let built_in_functions = HashMap::new();
  // Render that Vec<TemplateValue> as a Substack
  let rendered = render_template(
    Template::substack(parsed),
    &mut Vec::new(), // Current stack, not applicable
    &mut HashMap::new(), // Current local scope, not applicable
    &global_scope,
  );
  // The rendering output is the whole data stack initially
  // And an invoke the whole program
  let program_queue = vec![ProgramValue::Invoke].into();
  let mut data_stack = rendered;
  interpret(
    program_queue,
    &mut data_stack,
    global_scope,
    &built_in_functions,
  );
  println!("{:?}", data_stack);
}
