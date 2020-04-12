use serde_json::Value;
use valico::json_schema;
use url;

pub struct JSONValidator {
  scope: json_schema::scope::Scope,
  id: url::Url
}

impl JSONValidator {
  pub fn new(schema: &str) -> Self {
    let schema_json: Value = serde_json::from_str(schema).unwrap();
    let mut scope = json_schema::Scope::new();
    Self {
      id: scope.compile(schema_json.clone(), false).unwrap(),
      scope: scope,
    }
  }

  pub fn validate(&self, json_str: &str) -> Result<(), json_schema::ValidationState> {
    let schema = self.scope.resolve(&self.id).unwrap();
    let check_value = serde_json::from_str(json_str).unwrap();
    let state = schema.validate(&check_value);
    if state.is_valid() {
      Ok(())
    } else {
      Err(state)
    }
  }
}


