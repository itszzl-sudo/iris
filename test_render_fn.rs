use iris_sfc::compile;

fn main() {
    println!("Testing App.vue compilation...\n");
    
    match compile("examples/vue-demo/src/App.vue") {
        Ok(module) => {
            println!("✅ Compilation successful!\n");
            println!("Component name: {}", module.name);
            println!("Script length: {} bytes\n", module.script.len());
            println!("Render function length: {} bytes\n", module.render_fn.len());
            println!("=== Render Function ===\n");
            println!("{}", module.render_fn);
            println!("\n=== End Render Function ===\n");
            println!("=== Script ===\n");
            println!("{}", module.script);
            println!("\n=== End Script ===\n");
        }
        Err(e) => {
            println!("❌ Compilation failed: {}", e);
        }
    }
}
