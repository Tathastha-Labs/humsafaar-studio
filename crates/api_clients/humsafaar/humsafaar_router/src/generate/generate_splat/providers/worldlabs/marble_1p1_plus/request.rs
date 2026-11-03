use crate::client::router_worldlabs_client::RouterWorldLabsClient;
use crate::errors::humsafaar_router_error::HumsafaarRouterError;
use crate::generate::generate_splat::generate_splat_response::GenerateSplatResponse;
use crate::generate::generate_splat::providers::worldlabs::request_common::WorldLabsSplatRequest;

#[derive(Clone, Debug)]
pub struct WorldLabsMarble1p1PlusRequestState {
  /// Final materialized request; ready to fire. Media references (if any)
  /// have been uploaded as World Labs media assets by the draft phase.
  pub request: WorldLabsSplatRequest,
}

impl WorldLabsMarble1p1PlusRequestState {
  pub async fn send(&self, client: &RouterWorldLabsClient) -> Result<GenerateSplatResponse, HumsafaarRouterError> {
    self.request.send(client).await
  }
}
