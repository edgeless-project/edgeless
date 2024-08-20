#[derive(Debug, PartialEq, Eq, allocative::Allocative, starlark::any::ProvidesStaticType, serde::Serialize, serde::Deserialize, Clone)]
pub struct File {
    pub path: String,
}

starlark::starlark_simple_value!(File);

impl std::fmt::Display for File {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl<'v> starlark::values::UnpackValue<'v> for File {
    fn unpack_value(value: starlark::values::Value<'v>) -> Option<Self> {
        starlark::values::ValueLike::downcast_ref::<File>(value).cloned()
    }
}

#[starlark::values::starlark_value(type = "edgeless_file_c")]
impl<'v> starlark::values::StarlarkValue<'v> for File {}

#[starlark::starlark_module]
pub fn file(builder: &mut starlark::environment::GlobalsBuilder) {
    fn file<'v>(
        path: String,
        eval: &mut starlark::eval::Evaluator<'v, '_>,
        // heap: &'v starlark::values::Heap,
    ) -> anyhow::Result<starlark::values::Value<'v>> {
        let requested_path = std::path::PathBuf::from(path);

        let final_path = if requested_path.is_absolute() {
            requested_path.to_str().unwrap().to_string()
        } else {
            let ctx = eval.extra.unwrap().downcast_ref::<super::FileContext>().unwrap();
            ctx.0.parent().unwrap().join(requested_path).to_str().unwrap().to_string()
        };

        Ok(eval.heap().alloc(File { path: final_path }))
    }
}
