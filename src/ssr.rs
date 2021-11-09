use std::convert::TryFrom;

use v8::{script_compiler::Source, Context, DataError, Global, ModuleStatus, OwnedIsolate};

pub struct SsrEngine {
    isolate: OwnedIsolate,
    context: Global<Context>,
}

impl SsrEngine {
    pub fn init() {
        lazy_static! {
          static ref INIT_PLATFORM: () = {
              //Initialize a new V8 platform
              let platform = v8::new_default_platform(0,false).make_shared();
              v8::V8::initialize_platform(platform);
              v8::V8::initialize();
          };
        }

        lazy_static::initialize(&INIT_PLATFORM);
    }
    fn fake_script_origin<'s>(
        scope: &mut v8::HandleScope<'s>,
        resource_name_: &str,
    ) -> v8::ScriptOrigin<'s> {
        let resource_name = v8::String::new(scope, resource_name_).unwrap();
        let resource_line_offset = 0;
        let resource_column_offset = 0;
        let resource_is_shared_cross_origin = true;
        let script_id = 123;
        let source_map_url = v8::String::new(scope, "source_map_url").unwrap();
        let resource_is_opaque = true;
        let is_wasm = false;
        let is_module = true;
        v8::ScriptOrigin::new(
            scope,
            resource_name.into(),
            resource_line_offset,
            resource_column_offset,
            resource_is_shared_cross_origin,
            script_id,
            source_map_url.into(),
            resource_is_opaque,
            is_wasm,
            is_module,
        )
    }
    pub fn new() -> Self {
        let mut isolate = v8::Isolate::new(Default::default());
        let global_context;
        {
            // A stack-allocated class that governs a number of local handles.
            let handle_scope = &mut v8::HandleScope::new(&mut isolate);
            let context = v8::Context::new(handle_scope);
            //A sandboxed execution context with its own set of built-in objects and functions.
            global_context = Global::new(handle_scope, context);
        }
        Self {
            isolate,
            context: global_context,
        }
    }
    /// Evaluates the javascript source code passed and runs the render functions.
    /// Any initial params (if needed) must be passed as JSON.
    ///
    /// <a href="https://github.com/Valerioageno/ssr-rs/blob/main/examples/actix_with_initial_props.rs" target="_blank">Here</a> an useful example of how to use initial params with the actix framework.
    ///
    /// "enrty_point" is the variable name set from the frontend bundler used. <a href="https://github.com/Valerioageno/ssr-rs/blob/main/client/webpack.ssr.js" target="_blank">Here</a> an example from webpack.
    pub fn render_to_string(
        self: &mut Self,
        source: &str,
        params: Option<&str>,
    ) -> Result<String, String> {
        //The isolate rapresente an isolated instance of the v8 engine
        //Object from one isolate must not be used in other isolates.
        let isolate = &mut self.isolate;
        //A stack-allocated class that governs a number of local handles.
        let handle_scope = &mut v8::HandleScope::new(isolate);

        //A sandboxed execution context with its own set of built-in objects and functions.
        let context = v8::Local::new(handle_scope, &self.context);

        //Stack-allocated class which sets the execution context for all operations executed within a local scope.
        let scope = &mut v8::ContextScope::new(handle_scope, context);

        let code = v8::String::new(scope, source).expect("Strings are needed");
        let code = Source::new(code, Some(&Self::fake_script_origin(scope, "")));
        let module = v8::script_compiler::compile_module(scope, code)
            .ok_or("There aren't runnable scripts")?;
        let scope = &mut v8::HandleScope::new(scope);
        if matches!(
            module.instantiate_module(scope, |_, _, _, _| None),
            Some(true)
        ) {
            module.evaluate(scope).ok_or("failed to evaluate module")?;
            if module.get_status() == ModuleStatus::Errored {
                let exception = module
                    .get_exception()
                    .to_object(scope)
                    .ok_or("failed to convert exception to an object")?;
                let message_name =
                    v8::String::new(scope, "message").ok_or("failed to create message string")?;
                let exception_message = (*exception)
                    .get(scope, message_name.into())
                    .ok_or("failed to get exception message")?;
                return Err(exception_message.to_rust_string_lossy(scope));
            }
            let exports: v8::Local<v8::Object> = v8::Local::try_from(module.get_module_namespace())
                .map_err(|op: DataError| op.to_string())?;
            let default_export_name =
                v8::String::new(scope, "default").ok_or("failed to create default string")?;
            let default_export = (*exports)
                .get(scope, default_export_name.into())
                .ok_or("module have a default export")?;
            let func: v8::Local<v8::Function> = unsafe { v8::Local::cast(default_export) };

            //let func: v8::Local<v8::Function> = unsafe { v8::Local::cast(object) };
            let params: v8::Local<v8::Value> = match v8::String::new(scope, params.unwrap_or("")) {
                Some(s) => s.into(),
                None => v8::undefined(scope).into(),
            };

            let undef = v8::undefined(scope).into();
            let try_catch = &mut v8::TryCatch::new(scope);
            let result = func
                .call(try_catch, undef, &[params])
                .ok_or("default export is not a function")?;
            if try_catch.message().is_some() {
                let message = try_catch.message().unwrap().get(try_catch);
                return Err(message.to_rust_string_lossy(try_catch));
            }
            let rendered = result.to_rust_string_lossy(try_catch);
            Ok(rendered)
        } else {
            Err("failed to instantiate Module".to_string())
        }
    }
}
