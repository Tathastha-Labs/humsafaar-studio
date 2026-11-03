use crate::api::router_splat_model::RouterSplatModel;
use crate::api::image_list_ref::ImageListRef;
use crate::api::router_provider::RouterProvider;
use crate::errors::humsafaar_router_error::HumsafaarRouterError;
use crate::generate::generate_splat::splat_generation_plan::SplatGenerationPlan;

pub struct GenerateSplatRequest {
  /// Which model to use.
  pub model: RouterSplatModel,

  /// Which provider to use.
  pub provider: RouterProvider,

  /// The prompt for splat generation (optional).
  pub prompt: Option<String>,

  /// Reference images (optional).
  pub reference_images: Option<ImageListRef>,

  /// Some providers support idempotency.
  /// If not supplied, we'll generate one for the required providers.
  pub idempotency_token: Option<String>,
}

impl GenerateSplatRequest {
  /// Read the splat generation request, construct a plan, then yield a means to execute it.
  pub fn build(&self) -> Result<SplatGenerationPlan, HumsafaarRouterError> {
    match self.provider {
      _ => self.unsupported_provider(),
    }
  }


  fn unsupported_provider(&self) -> Result<SplatGenerationPlan, HumsafaarRouterError> {
    Err(HumsafaarRouterError::UnsupportedModel(
      format!("Splat generation for model `{:?}` is not supported for provider {:?}", self.model, self.provider)
    ))
  }

  pub fn get_or_generate_idempotency_token(&self) -> String {
    self.idempotency_token.clone()
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
  }
}
