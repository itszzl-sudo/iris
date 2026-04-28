#[test]
fn test_app_render_function() {
    use iris_sfc::compile;
    
    let result = compile("examples/vue-demo/src/App.vue");
    
    match result {
        Ok(module) => {
            println!("\n=== App.vue Compilation ===\n");
            println!("Component: {}", module.name);
            println!("Script size: {} bytes", module.script.len());
            println!("Render function size: {} bytes\n", module.render_fn.len());
            
            println!("=== Render Function ===");
            println!("{}", module.render_fn);
            println!("\n=== End Render Function ===\n");
            
            println!("=== Script ===");
            println!("{}", module.script);
            println!("\n=== End Script ===\n");
            
            assert!(module.render_fn.contains("function render()"));
        }
        Err(e) => {
            panic!("Compilation failed: {}", e);
        }
    }
}
