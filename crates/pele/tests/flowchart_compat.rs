//! Public feature-slice integration: flowchart source preparation, parser and ordered models.
use pele::compat::{diagrams::flowchart,ParseOptions};
use serde_json::Value;
#[test]
fn pinned_flowchart_parser_corpus(){
 let fixture:Value=serde_json::from_str(include_str!("../../../fixtures/pele/flowchart.json")).unwrap();
 let cases=fixture["cases"].as_array().unwrap();let mut failures=Vec::new();
 for case in cases{let options:ParseOptions=serde_json::from_value(case["options"].clone()).unwrap();let actual=match flowchart::parse(case["source"].as_str().unwrap(),&options){Ok(model)=>serde_json::json!({"model":model}),Err(error)=>serde_json::json!({"error":error})};let mut expected=case["expected"].clone();expected.as_object_mut().unwrap().remove("result");if actual!=expected{if failures.len()<10{eprintln!("{} source={:?}\nactual={}\nexpected={}",case["id"],case["source"],actual,expected)}failures.push(case["id"].clone())}}
 assert!(failures.is_empty(),"{} / {} mismatches: {:?}",failures.len(),cases.len(),failures.get(..10.min(failures.len())).unwrap());
}
