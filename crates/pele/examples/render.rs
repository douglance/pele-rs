//! `cargo run -p pele-rs --example render -- SOURCE.mmd OUTPUT.svg`
use pele::compat::{text::MetricsResources, Engine, RenderOptions};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 {
        return Err("Usage: render SOURCE.mmd OUTPUT.svg".into());
    }
    let source = std::fs::read_to_string(&args[1])?;
    let drawing = Engine::with_builtins().render_source(
        &source,
        &RenderOptions::default(),
        &mut MetricsResources,
    )?;
    std::fs::write(&args[2], drawing.rendered.svg)?;
    println!("{} × {}", drawing.rendered.width, drawing.rendered.height);
    Ok(())
}
