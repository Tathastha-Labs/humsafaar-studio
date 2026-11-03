use crate::errors::humsafaar_router_error::HumsafaarRouterError;
use crate::generate::generate_splat::providers::worldlabs::draft_common::WorldLabsSplatDraft;
use crate::generate::generate_splat::providers::worldlabs::marble_1p0::request::WorldLabsMarble1p0RequestState;
use crate::generate::generate_splat::splat_generation_draft_context::SplatGenerationDraftContext;

#[derive(Clone, Debug)]
pub struct WorldLabsMarble1p0DraftState {
  /// Shared World Labs draft; the model is baked in at build time.
  pub draft: WorldLabsSplatDraft,
}

impl WorldLabsMarble1p0DraftState {
  pub async fn to_request(&self, draft_context: &SplatGenerationDraftContext<'_>) -> Result<WorldLabsMarble1p0RequestState, HumsafaarRouterError> {
    let request = self.draft.to_request(draft_context).await?;
    Ok(WorldLabsMarble1p0RequestState { request })
  }
}
