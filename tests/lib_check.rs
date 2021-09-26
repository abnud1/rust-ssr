use rust_ssr::SsrEngine;

#[test]
fn incorrect_entry_point() {
    let source = r##"var entryPoint = {x: () => "<html></html>"};"##;
    SsrEngine::init();
    let mut ssr = SsrEngine::new();
    let result = ssr.render_to_string(&source, "IncorrectEntryPoint", None);
    assert_eq!(
        result,
        Err("Missing entry point. Is the bundle exported as a variable?".into())
    );
}

#[test]
fn pass_param_to_function() {
    let props = r#"{"Hello world"}"#;

    let source = r##"var SSR = {x: (params) => "These are our parameters: " + params};"##;
    SsrEngine::init();

    let mut ssr = SsrEngine::new();
    let result = ssr.render_to_string(&source, "SSR", Some(&props));

    assert_eq!(
        result,
        Ok("These are our parameters: {\"Hello world\"}".into())
    );

    let source2 = r##"var SSR = {x: () => "I don't accept params"};"##;

    let result2 = ssr.render_to_string(&source2, "SSR", Some(&props));

    assert_eq!(result2, Ok("I don't accept params".into()));
}

#[test]
fn render_simple_html() {
    let source = r##"var SSR = {x: () => "<html></html>"};"##;
    SsrEngine::init();
    let mut ssr = SsrEngine::new();

    let html = ssr.render_to_string(&source, "SSR", None);

    assert_eq!(html, Ok("<html></html>".into()));

    //Prevent missing semicolon
    let source2 = r##"var SSR = {x: () => "<html></html>"}"##;

    let html2 = ssr.render_to_string(&source2, "SSR", None);

    assert_eq!(html2, Ok("<html></html>".into()));
}
