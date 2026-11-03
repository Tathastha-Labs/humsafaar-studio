use super::request::OmniRequest;
use crate::core::commands::response::failure_response_wrapper::{CommandErrorResponseWrapper, CommandErrorStatus};
use crate::core::commands::response::shorthand::Response;
use crate::core::commands::response::success_response_wrapper::SerializeMarker;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Deserialize, Serialize)]
#[serde(transparent)]
pub struct OmniResponse(pub Value);

impl SerializeMarker for OmniResponse {}

pub type OmniResult = Response<OmniResponse, String, ()>;

/// Answer in the shape the generate pages expect, whatever the handler returned.
pub fn adapt_legacy_response<T: Serialize, E: Serialize, P: Serialize>(response: Response<T, E, P>) -> OmniResult {
  match response {
    Ok(response) => Ok(OmniResponse(serde_json::to_value(response.payload).map_err(local_error)?).into()),
    Err(err) => Err(CommandErrorResponseWrapper {
      status: err.status,
      error_message: err.error_message.or_else(|| {
        serde_json::to_value(err.error_details).ok()
          .and_then(|value| value.get("error_message").and_then(Value::as_str).map(str::to_owned))
      }),
      error_type: err.error_type
        .and_then(|value| serde_json::to_value(value).ok())
        .and_then(|value| value.as_str().map(str::to_owned)),
      error_details: None,
    }),
  }
}

pub fn decode_native<T: serde::de::DeserializeOwned>(request: OmniRequest) -> Result<T, CommandErrorResponseWrapper<String, ()>> {
  serde_json::from_value(serde_json::to_value(request).map_err(local_error)?)
    .map_err(|err| command_error(CommandErrorStatus::BadRequest, "bad_input", err.to_string()))
}

pub fn local_error(err: impl std::fmt::Display) -> CommandErrorResponseWrapper<String, ()> {
  command_error(CommandErrorStatus::ServerError, "server_error", err.to_string())
}

fn command_error(status: CommandErrorStatus, kind: &str, message: String) -> CommandErrorResponseWrapper<String, ()> {
  CommandErrorResponseWrapper { status, error_type: Some(kind.into()), error_message: Some(message), error_details: None }
}
