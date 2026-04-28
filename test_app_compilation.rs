use iris_sfc::compile;

fn main() {
    println!("Testing App.vue compilation...\n");
    
    match compile("examples/vue-demo/src/App.vue") {
        Ok(module) => {
            println!("✅ Compilation successful!\n");
            println!("Component name: {}", module.name);
            println!("Script length: {} bytes\n", module.script.len());
            println!("=== Compiled Script ===\n");
            println!("{}", module.script);
            println!("\n=== End Script ===\n");
            println!("Styles count: {}", module.styles.len());
        }
        Err(e) => {
            println!("❌ Compilation failed: {}", e);
        }
    }
}
