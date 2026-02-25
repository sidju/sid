use clap::Parser;
use sid::{parse_str, llvm_backend};

#[derive(Parser)]
struct Args {
    /// Source file to parse (omit to run the hard-coded demo)
    source: Option<String>,

    /// Print LLVM IR to stdout instead of writing an object file
    #[arg(long)]
    emit_llvm: bool,

    /// Output object file path
    #[arg(short, long, default_value = "out.o")]
    out: String,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    if let Some(path) = &args.source {
        let source = std::fs::read_to_string(path)?;
        let (program, consumed) = parse_str(&source)?;
        println!("Parsed {} program value(s), consumes {} parent-stack entries.", program.len(), consumed);
        for (i, v) in program.iter().enumerate() {
            println!("  [{i}] {v:?}");
        }
        return Ok(());
    }

    let module = llvm_backend::compile_demo_module("sid_demo")?;

    if args.emit_llvm {
        println!("{}", module.print_to_string().to_string());
    } else {
        llvm_backend::emit_object_file(&module, &args.out)?;
        println!("Object file written to {}", args.out);
    }

    Ok(())
}
