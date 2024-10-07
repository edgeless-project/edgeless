use std::str::FromStr;

pub mod actor;
pub mod actor_class;
pub mod files;
pub mod inner_structure;
pub mod port;
pub mod port_class;
pub mod resource;
pub mod resource_class;
pub mod workflow;

#[derive(Debug)]
pub enum LoadResult {
    Workflow(crate::workflow::EdgelessWorkflow),
    ActorClass(crate::actor_class::EdgelessActorClass),
}

#[derive(Debug, starlark::any::ProvidesStaticType, Default)]
struct FileContext(std::path::PathBuf);

pub fn load(main_file: std::path::PathBuf) -> anyhow::Result<LoadResult> {
    let m = load_module(&main_file).map_err(|e| anyhow::anyhow!(e))?;

    if let Ok(main) = m.get("el_main") {
        if let Ok(workflow) = main.clone().downcast::<crate::workflow::EdgelessWorkflow>() {
            // panic!("{:?}", workflow);
            return Ok(LoadResult::Workflow(workflow.as_ref().clone()));
        }

        if let Ok(actor) = main.downcast::<crate::actor_class::EdgelessActorClass>() {
            return Ok(LoadResult::ActorClass(actor.as_ref().clone()));
        }
    }

    return Err(anyhow::anyhow!("Tried to load unknown entity!"));
}

fn load_module(file: &std::path::PathBuf) -> starlark::Result<starlark::environment::FrozenModule> {
    let filename: String = String::from_str(file.file_name().unwrap().to_str().unwrap()).map_err(|e| anyhow::anyhow!(e))?;
    let parent = file.parent().unwrap().to_owned();

    let data = std::fs::read_to_string(file).map_err(|e| anyhow::anyhow!(e))?;

    let ast = starlark::syntax::AstModule::parse(&filename, data, &starlark::syntax::Dialect::Standard).unwrap();

    let mut loads = std::collections::HashMap::new();

    for load in ast.loads() {
        loads.insert(
            load.module_id.to_owned(),
            load_module(&parent.join(std::path::PathBuf::from_str(load.module_id).unwrap()))?,
        );
    }

    let load_refs = loads.iter().map(|(k, v)| (k.as_str(), v)).collect();

    let mut loader = starlark::eval::ReturnFileLoader { modules: &load_refs };

    let globals = starlark::environment::GlobalsBuilder::extended_by(&[starlark::environment::LibraryExtension::Print])
        .with(crate::inner_structure::edgeless_inner_structure)
        .with(crate::actor_class::edgeless_actor_class)
        .with(crate::resource_class::edgeless_resource_class)
        .with(crate::actor::edgeless_actor)
        .with(crate::resource::edgeless_resource)
        .with(crate::port_class::edgeless_port_spec)
        .with(crate::workflow::edgeless_workflow)
        .with(crate::files::file)
        .build();

    let module = starlark::environment::Module::new();
    let context = FileContext(file.canonicalize().unwrap());

    {
        let mut eval = starlark::eval::Evaluator::new(&module);
        eval.set_loader(&mut loader);
        eval.extra = Some(&context);
        eval.eval_module(ast, &globals).unwrap();
    }

    Ok(module.freeze()?)
}
