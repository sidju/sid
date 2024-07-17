use std::collections::HashMap;

use sid::*;

use clap::Parser;

#[cfg(feature= "debug-gui")]
mod debug_gui;

#[derive(Parser)]
#[command(version)]
struct CliArgs {
  /// Path to file to execute code from or `-` for stdin
  file: String,
}

#[cfg(not(feature = "debug-gui"))]
fn main() {
  let cli = CliArgs::parse();
  run_file(&cli.file.clone());
}


#[cfg(feature = "debug-gui")]
#[cfg(target_arch = "wasm32")]
fn main() {
    // Make sure panics are logged using `console.error`.
    console_error_panic_hook::set_once();

    // Redirect tracing to console.log and friends:
    tracing_wasm::set_as_global_default();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        eframe::start_web(
            "canvas",
            web_options,
            Box::new(|_cc| Box::new(debug_gui::SidDebuggerApp::new())),
        )
        .await
        .expect("failed to start eframe");
    });
}

#[cfg(feature = "debug-gui")]
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let options = eframe::NativeOptions {
        ..Default::default()
    };
    let res = eframe::run_native(
        "Sid - Debugger",
        options,
        Box::new(|_cc| Box::new(debug_gui::SidDebuggerApp::new())),
    );

    if res.is_err() {
        println!("Error: {:?}", res);
    }
}

fn run_file(path: &str) {
  // Create a String from the file
  let file_content = std::fs::read_to_string(path)
    .expect("Failed to read file");
  run(&file_content);
}

struct Program {
  instructions: Vec<DataValue>,
  global_scope: HashMap<String, RealValue>,
}

fn compile(source: & str) -> Program {
  // Parse that String into Vec<TemplateValue>
  let parsed = parse_str(source);
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
