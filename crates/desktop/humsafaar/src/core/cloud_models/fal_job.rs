use serde_json::Value;

/// One request to a Fal endpoint: where it goes and what is sent.
#[derive(Debug, PartialEq)]
pub struct FalJob {
  pub endpoint: &'static str,
  pub input: Value,
}
