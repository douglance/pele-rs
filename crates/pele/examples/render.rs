//! `cargo run -p pele-rs --example render -- SOURCE.mmd OUTPUT.svg`
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 {
        return Err("Usage: render SOURCE.mmd OUTPUT.svg".into());
    }
    let source = std::fs::read_to_string(&args[1])?;
    let result = pele::render(&source, &pele::RenderOptions::default())?;
    std::fs::write(&args[2], result.svg)?;
    println!(
        "{} × {}: {} nodes, {} edges",
        result.width,
        result.height,
        result.geometry.nodes.len(),
        result.geometry.edges.len()
    );
    Ok(())
}
