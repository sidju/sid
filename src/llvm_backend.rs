use anyhow::Context;
use inkwell::{
    module::Module,
    targets::{
        CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine,
    },
    OptimizationLevel,
};

pub fn compile_demo_module(module_name: &str) -> anyhow::Result<Module<'static>> {
    let context = Box::leak(Box::new(inkwell::context::Context::create()));
    let module = context.create_module(module_name);
    let builder = context.create_builder();

    let i64_type = context.i64_type();

    // Define `add(i64, i64) -> i64`
    let add_fn_type = i64_type.fn_type(&[i64_type.into(), i64_type.into()], false);
    let add_fn = module.add_function("add", add_fn_type, None);
    let add_entry = context.append_basic_block(add_fn, "entry");
    builder.position_at_end(add_entry);
    let a = add_fn.get_nth_param(0).unwrap().into_int_value();
    let b = add_fn.get_nth_param(1).unwrap().into_int_value();
    let sum = builder.build_int_add(a, b, "sum")?;
    builder.build_return(Some(&sum))?;

    // Define `main() -> i64` that returns `add(40, 2)`
    let main_fn_type = i64_type.fn_type(&[], false);
    let main_fn = module.add_function("main", main_fn_type, None);
    let main_entry = context.append_basic_block(main_fn, "entry");
    builder.position_at_end(main_entry);
    let forty = i64_type.const_int(40, false);
    let two = i64_type.const_int(2, false);
    let result = builder
        .build_call(add_fn, &[forty.into(), two.into()], "result")?
        .try_as_basic_value()
        .left()
        .context("add() returned void")?;
    builder.build_return(Some(&result))?;

    Ok(module)
}

pub fn emit_object_file(module: &Module, out_path: &str) -> anyhow::Result<()> {
    Target::initialize_all(&InitializationConfig::default());

    let triple = TargetMachine::get_default_triple();
    let cpu_cstr = TargetMachine::get_host_cpu_name();
    let features_cstr = TargetMachine::get_host_cpu_features();
    let cpu = cpu_cstr.to_str().context("host CPU name is not valid UTF-8")?;
    let features = features_cstr
        .to_str()
        .context("host CPU features string is not valid UTF-8")?;

    let target = Target::from_triple(&triple).map_err(|e| anyhow::anyhow!("{}", e))?;
    let machine = target
        .create_target_machine(
            &triple,
            cpu,
            features,
            OptimizationLevel::Default,
            RelocMode::PIC,
            CodeModel::Default,
        )
        .context("failed to create target machine")?;

    module.set_triple(&triple);
    module.set_data_layout(&machine.get_target_data().get_data_layout());

    machine
        .write_to_file(module, FileType::Object, std::path::Path::new(out_path))
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    Ok(())
}
