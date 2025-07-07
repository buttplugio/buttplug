const SCHEMA_DIR: &str = "./schema/";
use jsonschema::Validator;

fn main() {
  println!("cargo:rerun-if-changed={}", SCHEMA_DIR); 
  let schema: serde_json::Value =
    serde_json::from_str(&std::fs::read_to_string(std::path::Path::new(SCHEMA_DIR).join("buttplug-schema.json")).unwrap()).expect("Built in schema better be valid json");
  let _ = Validator::new(&schema).expect("Built in schema better be a valid schema");
}