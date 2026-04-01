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
    let file_content = std::fs::read_to_string(path).expect("Failed to read file");
    run(&file_content);
}

fn compile(source: &str) -> Program {
    let parsed = parse_str(source).expect("parse error");
    let mut global_scope = default_scope();
    let comptime_builtins = get_comptime_builtins();
    let after_comptime =
        comptime_pass(parsed.0, &comptime_builtins, &mut global_scope).expect("comptime error");
    let rendered = {
        let mut gs = GlobalState::new(&mut global_scope);
        render_template(
            Template::substack((after_comptime, 0)),
            &mut Vec::new(),
            &HashMap::new(),
            &mut gs,
            &comptime_builtins,
        )
    };
    // render_template returns a DataValue; lift it into TemplateValue.
    let instructions = vec![TemplateValue::from(rendered)];
    Program {
        instructions,
        global_scope,
    }
}

fn run(source: &str) {
    let program = compile(source);
    let mut global_scope = program.global_scope;
    let global_state = GlobalState::new(&mut global_scope);
    let program_stack = vec![ProgramValue::Invoke];
    let runtime_builtins = get_interpret_builtins();
    interpret(
        program_stack,
        program.instructions,
        global_state,
        &runtime_builtins,
    );
}
