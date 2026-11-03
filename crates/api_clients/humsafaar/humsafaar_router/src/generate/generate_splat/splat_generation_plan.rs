use crate::client::router_client::RouterClient;
use crate::errors::humsafaar_router_error::HumsafaarRouterError;
use crate::generate::generate_splat::generate_splat_response::GenerateSplatResponse;
use crate::generate::generate_splat::splat_generation_cost_estimate::SplatGenerationCostEstimate;

#[derive(Debug)]
pub enum SplatGenerationPlan {
}

impl SplatGenerationPlan {
  pub async fn generate_splat(
    &self,
    client: &RouterClient,
  ) -> Result<GenerateSplatResponse, HumsafaarRouterError> {
    match *self {}
  }

  pub fn estimate_costs(&self) -> SplatGenerationCostEstimate {
    match *self {}
  }
}
