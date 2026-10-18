//! Native reference results for the cross-language integration corpus.
use serde_json::Value;
use std::{fs, path::PathBuf};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let dir = root.join("dist/evidence/rust");
    fs::create_dir_all(&dir)?;
    let fixtures: Vec<Value> =
        serde_json::from_str(&fs::read_to_string(root.join("fixtures/manifest.json"))?)?;
    for f in fixtures {
        let source = match f["source"].as_str() {
            Some(s) => s.to_owned(),
            None => fs::read_to_string(root.join("fixtures").join(f["file"].as_str().unwrap()))?,
        };
        let result = pele::render_json(&source, "");
        let value: Value = serde_json::from_str(&result)?;
        if f.get("error").is_none() {
            assert_eq!(value["ok"], true, "{}", f["name"]);
        }
        fs::write(
            dir.join(format!("{}.json", f["name"].as_str().unwrap())),
            result,
        )?;
    }
    println!("Native Rust corpus: {}", dir.display());
    Ok(())
}
