fn main() -> Result<(), Box<dyn std::error::Error>> {
    let diagram = pele::render("graph TD; A[Packaged crate] --> B{Usable?} --> C([Yes])", &Default::default())?;
    assert_eq!(diagram.geometry.nodes.len(), 3);
    assert_eq!(diagram.geometry.edges.len(), 2);
    assert!(diagram.svg.contains("<path"));
    println!("Packaged Rust crate consumer passed: {} × {}", diagram.width, diagram.height);
    Ok(())
}
