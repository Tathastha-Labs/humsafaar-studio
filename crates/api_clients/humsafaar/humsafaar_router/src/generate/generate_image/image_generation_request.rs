use crate::generate::generate_image::providers::midjourney::request::MidjourneyRequestState;
use crate::generate::generate_image::providers::midjourney::cost::MidjourneyCostState;
use crate::api::router_provider::RouterProvider;
use crate::client::router_client::RouterClient;
use crate::errors::humsafaar_router_error::HumsafaarRouterError;
use crate::generate::generate_image::generate_image_response::GenerateImageResponse;
use crate::generate::generate_image::image_generation_cost_estimate::ImageGenerationCostEstimate;
use crate::generate::generate_image::providers::fal::flux_1_dev::cost::FalFlux1DevCostState;
use crate::generate::generate_image::providers::fal::flux_1_dev::request::FalFlux1DevRequestState;
use crate::generate::generate_image::providers::fal::flux_1_schnell::cost::FalFlux1SchnellCostState;
use crate::generate::generate_image::providers::fal::flux_1_schnell::request::FalFlux1SchnellRequestState;
use crate::generate::generate_image::providers::fal::flux_2_lora_angles::cost::FalFlux2LoraAnglesCostState;
use crate::generate::generate_image::providers::fal::flux_2_lora_angles::request::FalFlux2LoraAnglesRequestState;
use crate::generate::generate_image::providers::fal::flux_pro_1p1::cost::FalFluxPro1p1CostState;
use crate::generate::generate_image::providers::fal::flux_pro_1p1::request::FalFluxPro1p1RequestState;
use crate::generate::generate_image::providers::fal::flux_pro_1p1_ultra::cost::FalFluxPro1p1UltraCostState;
use crate::generate::generate_image::providers::fal::flux_pro_1p1_ultra::request::FalFluxPro1p1UltraRequestState;
use crate::generate::generate_image::providers::fal::gpt_image_1::cost::FalGptImage1CostState;
use crate::generate::generate_image::providers::fal::gpt_image_1::request::FalGptImage1RequestState;
use crate::generate::generate_image::providers::fal::gpt_image_1p5::cost::FalGptImage1p5CostState;
use crate::generate::generate_image::providers::fal::gpt_image_1p5::request::FalGptImage1p5RequestState;
use crate::generate::generate_image::providers::fal::gpt_image_2::cost::FalGptImage2CostState;
use crate::generate::generate_image::providers::fal::gpt_image_2::request::FalGptImage2RequestState;
use crate::generate::generate_image::providers::fal::gpt_image_2p5_flare::cost::FalGptImage2p5FlareCostState;
use crate::generate::generate_image::providers::fal::gpt_image_2p5_flare::request::FalGptImage2p5FlareRequestState;
use crate::generate::generate_image::providers::fal::gpt_image_2p5_sunburst::cost::FalGptImage2p5SunburstCostState;
use crate::generate::generate_image::providers::fal::gpt_image_2p5_sunburst::request::FalGptImage2p5SunburstRequestState;
use crate::generate::generate_image::providers::fal::nano_banana::cost::FalNanoBananaCostState;
use crate::generate::generate_image::providers::fal::nano_banana::request::FalNanoBananaRequestState;
use crate::generate::generate_image::providers::fal::nano_banana_2::cost::FalNanoBanana2CostState;
use crate::generate::generate_image::providers::fal::nano_banana_2::request::FalNanoBanana2RequestState;
use crate::generate::generate_image::providers::fal::nano_banana_pro::cost::FalNanoBananaProCostState;
use crate::generate::generate_image::providers::fal::nano_banana_pro::request::FalNanoBananaProRequestState;
use crate::generate::generate_image::providers::fal::qwen_edit_2511_angles::cost::FalQwenEdit2511AnglesCostState;
use crate::generate::generate_image::providers::fal::qwen_edit_2511_angles::request::FalQwenEdit2511AnglesRequestState;
use crate::generate::generate_image::providers::fal::seedream_4::cost::FalSeedream4CostState;
use crate::generate::generate_image::providers::fal::seedream_4::request::FalSeedream4RequestState;
use crate::generate::generate_image::providers::fal::seedream_4p5::cost::FalSeedream4p5CostState;
use crate::generate::generate_image::providers::fal::seedream_4p5::request::FalSeedream4p5RequestState;
use crate::generate::generate_image::providers::fal::seedream_5_lite::cost::FalSeedream5LiteCostState;
use crate::generate::generate_image::providers::fal::seedream_5_lite::request::FalSeedream5LiteRequestState;
use crate::generate::generate_image::providers::kinovi::midjourney_7::cost::KinoviMidjourney7CostState;
use crate::generate::generate_image::providers::kinovi::midjourney_7::request::KinoviMidjourney7RequestState;
use crate::generate::generate_image::providers::kinovi::midjourney_7_niji::cost::KinoviMidjourney7NijiCostState;
use crate::generate::generate_image::providers::kinovi::midjourney_7_niji::request::KinoviMidjourney7NijiRequestState;
use crate::generate::generate_image::providers::kinovi::midjourney_8::cost::KinoviMidjourney8CostState;
use crate::generate::generate_image::providers::kinovi::midjourney_8::request::KinoviMidjourney8RequestState;
use crate::generate::generate_image::providers::kinovi::seedream_5p0_pro::cost::KinoviSeedream5p0ProCostState;
use crate::generate::generate_image::providers::kinovi::seedream_5p0_pro::request::KinoviSeedream5p0ProRequestState;

#[derive(Clone, Debug)]
pub enum ImageGenerationRequest {
  MidjourneyRequest(MidjourneyRequestState),
  // ── Humsafaar provider (omni-gen image endpoint) ──

  // ── Fal provider ──
  FalFlux1Dev(FalFlux1DevRequestState),
  FalFlux1Schnell(FalFlux1SchnellRequestState),
  FalFluxPro1p1(FalFluxPro1p1RequestState),
  FalFluxPro1p1Ultra(FalFluxPro1p1UltraRequestState),
  FalGptImage1(FalGptImage1RequestState),
  FalGptImage1p5(FalGptImage1p5RequestState),
  FalGptImage2(FalGptImage2RequestState),
  FalGptImage2p5Flare(FalGptImage2p5FlareRequestState),
  FalGptImage2p5Sunburst(FalGptImage2p5SunburstRequestState),
  FalNanoBanana(FalNanoBananaRequestState),
  FalNanoBanana2(FalNanoBanana2RequestState),
  FalNanoBananaPro(FalNanoBananaProRequestState),
  FalSeedream4(FalSeedream4RequestState),
  FalSeedream4p5(FalSeedream4p5RequestState),
  FalSeedream5Lite(FalSeedream5LiteRequestState),
  FalQwenEdit2511Angles(FalQwenEdit2511AnglesRequestState),
  FalFlux2LoraAngles(FalFlux2LoraAnglesRequestState),

  // ── Kinovi / KinoviWeb provider (Midjourney + Seedream image generation) ──
  KinoviMidjourney7(KinoviMidjourney7RequestState),
  KinoviMidjourney7Niji(KinoviMidjourney7NijiRequestState),
  KinoviMidjourney8(KinoviMidjourney8RequestState),
  KinoviSeedream5p0Pro(KinoviSeedream5p0ProRequestState),
}

impl ImageGenerationRequest {
  pub fn get_provider(&self) -> RouterProvider {
    match self {
      Self::MidjourneyRequest(_) => RouterProvider::Midjourney,

      Self::FalFlux1Dev(_) => RouterProvider::Fal,
      Self::FalFlux1Schnell(_) => RouterProvider::Fal,
      Self::FalFluxPro1p1(_) => RouterProvider::Fal,
      Self::FalFluxPro1p1Ultra(_) => RouterProvider::Fal,
      Self::FalGptImage1(_) => RouterProvider::Fal,
      Self::FalGptImage1p5(_) => RouterProvider::Fal,
      Self::FalGptImage2(_) => RouterProvider::Fal,
      Self::FalGptImage2p5Flare(_) => RouterProvider::Fal,
      Self::FalGptImage2p5Sunburst(_) => RouterProvider::Fal,
      Self::FalNanoBanana(_) => RouterProvider::Fal,
      Self::FalNanoBanana2(_) => RouterProvider::Fal,
      Self::FalNanoBananaPro(_) => RouterProvider::Fal,
      Self::FalSeedream4(_) => RouterProvider::Fal,
      Self::FalSeedream4p5(_) => RouterProvider::Fal,
      Self::FalSeedream5Lite(_) => RouterProvider::Fal,
      Self::FalQwenEdit2511Angles(_) => RouterProvider::Fal,
      Self::FalFlux2LoraAngles(_) => RouterProvider::Fal,

      Self::KinoviMidjourney7(_) => RouterProvider::KinoviWeb,
      Self::KinoviMidjourney7Niji(_) => RouterProvider::KinoviWeb,
      Self::KinoviMidjourney8(_) => RouterProvider::KinoviWeb,
      Self::KinoviSeedream5p0Pro(_) => RouterProvider::KinoviWeb,
    }
  }

  pub fn estimate_cost(&self) -> Result<ImageGenerationCostEstimate, HumsafaarRouterError> {
    match self {
      Self::MidjourneyRequest(request) => Ok(MidjourneyCostState::from_request(request).estimate_cost()),
      // ── Humsafaar ──

      // ── Fal ──
      Self::FalFlux1Dev(request) => Ok(FalFlux1DevCostState::from_request(request).estimate_cost()),
      Self::FalFlux1Schnell(request) => Ok(FalFlux1SchnellCostState::from_request(request).estimate_cost()),
      Self::FalFluxPro1p1(request) => Ok(FalFluxPro1p1CostState::from_request(request).estimate_cost()),
      Self::FalFluxPro1p1Ultra(request) => Ok(FalFluxPro1p1UltraCostState::from_request(request).estimate_cost()),
      Self::FalGptImage1(request) => Ok(FalGptImage1CostState::from_request(request).estimate_cost()),
      Self::FalGptImage1p5(request) => Ok(FalGptImage1p5CostState::from_request(request).estimate_cost()),
      Self::FalGptImage2(request) => Ok(FalGptImage2CostState::from_request(request).estimate_cost()),
      Self::FalGptImage2p5Flare(request) => Ok(FalGptImage2p5FlareCostState::from_request(request).estimate_cost()),
      Self::FalGptImage2p5Sunburst(request) => Ok(FalGptImage2p5SunburstCostState::from_request(request).estimate_cost()),
      Self::FalNanoBanana(request) => Ok(FalNanoBananaCostState::from_request(request).estimate_cost()),
      Self::FalNanoBanana2(request) => Ok(FalNanoBanana2CostState::from_request(request).estimate_cost()),
      Self::FalNanoBananaPro(request) => Ok(FalNanoBananaProCostState::from_request(request).estimate_cost()),
      Self::FalSeedream4(request) => Ok(FalSeedream4CostState::from_request(request).estimate_cost()),
      Self::FalSeedream4p5(request) => Ok(FalSeedream4p5CostState::from_request(request).estimate_cost()),
      Self::FalSeedream5Lite(request) => Ok(FalSeedream5LiteCostState::from_request(request).estimate_cost()),
      Self::FalQwenEdit2511Angles(request) => Ok(FalQwenEdit2511AnglesCostState::from_request(request).estimate_cost()),
      Self::FalFlux2LoraAngles(request) => Ok(FalFlux2LoraAnglesCostState::from_request(request).estimate_cost()),

      Self::KinoviMidjourney7(request) => Ok(KinoviMidjourney7CostState::from_request(request).estimate_cost()),
      Self::KinoviMidjourney7Niji(request) => Ok(KinoviMidjourney7NijiCostState::from_request(request).estimate_cost()),
      Self::KinoviMidjourney8(request) => Ok(KinoviMidjourney8CostState::from_request(request).estimate_cost()),
      Self::KinoviSeedream5p0Pro(request) => Ok(KinoviSeedream5p0ProCostState::from_request(request).estimate_cost()),
    }
  }

  pub async fn send_request(&self, client: &RouterClient) -> Result<GenerateImageResponse, HumsafaarRouterError> {
    match self {
      Self::MidjourneyRequest(request) => request.send(client.get_midjourney_client_ref()?).await,
      // ── Humsafaar (omni-gen image endpoint) ──

      // ── Fal ──
      Self::FalFlux1Dev(request) => {
        let fal_client = client.get_fal_client_ref()?;
        request.send(fal_client).await
      }
      Self::FalFlux1Schnell(request) => {
        let fal_client = client.get_fal_client_ref()?;
        request.send(fal_client).await
      }
      Self::FalFluxPro1p1(request) => {
        // Flux Pro 1.1 is webhook-required — uses the webhook-only RouterFalClient.
        let fal_client = client.get_fal_client_ref()?;
        request.send(fal_client).await
      }
      Self::FalFluxPro1p1Ultra(request) => {
        let fal_client = client.get_fal_client_ref()?;
        request.send(fal_client).await
      }
      Self::FalGptImage1(request) => {
        let fal_client = client.get_fal_client_ref()?;
        request.send(fal_client).await
      }
      Self::FalGptImage1p5(request) => {
        let fal_client = client.get_fal_client_ref()?;
        request.send(fal_client).await
      }
      Self::FalGptImage2(request) => {
        let fal_client = client.get_fal_client_ref()?;
        request.send(fal_client).await
      }
      Self::FalGptImage2p5Flare(request) => {
        let fal_client = client.get_fal_client_ref()?;
        request.send(fal_client).await
      }
      Self::FalGptImage2p5Sunburst(request) => {
        let fal_client = client.get_fal_client_ref()?;
        request.send(fal_client).await
      }
      Self::FalNanoBanana(request) => {
        // nano_banana (Gemini 2.5 Flash) is webhook-required.
        let fal_client = client.get_fal_client_ref()?;
        request.send(fal_client).await
      }
      Self::FalNanoBanana2(request) => {
        let fal_client = client.get_fal_client_ref()?;
        request.send(fal_client).await
      }
      Self::FalNanoBananaPro(request) => {
        let fal_client = client.get_fal_client_ref()?;
        request.send(fal_client).await
      }
      Self::FalSeedream4(request) => {
        // Seedream v4 is webhook-required.
        let fal_client = client.get_fal_client_ref()?;
        request.send(fal_client).await
      }
      Self::FalSeedream4p5(request) => {
        let fal_client = client.get_fal_client_ref()?;
        request.send(fal_client).await
      }
      Self::FalSeedream5Lite(request) => {
        let fal_client = client.get_fal_client_ref()?;
        request.send(fal_client).await
      }
      Self::FalQwenEdit2511Angles(request) => {
        // Webhook-required endpoint.
        let fal_client = client.get_fal_client_ref()?;
        request.send(fal_client).await
      }
      Self::FalFlux2LoraAngles(request) => {
        // Webhook-required endpoint.
        let fal_client = client.get_fal_client_ref()?;
        request.send(fal_client).await
      }

      // ── Kinovi / KinoviWeb (Midjourney + Seedream) ──
      Self::KinoviMidjourney7(request) => {
        let seedance_client = client.get_kinovi_web_client_ref()
          .map_err(HumsafaarRouterError::Client)?;
        request.send(seedance_client).await
      }
      Self::KinoviMidjourney7Niji(request) => {
        let seedance_client = client.get_kinovi_web_client_ref()
          .map_err(HumsafaarRouterError::Client)?;
        request.send(seedance_client).await
      }
      Self::KinoviMidjourney8(request) => {
        let seedance_client = client.get_kinovi_web_client_ref()
          .map_err(HumsafaarRouterError::Client)?;
        request.send(seedance_client).await
      }
      Self::KinoviSeedream5p0Pro(request) => {
        let seedance_client = client.get_kinovi_web_client_ref()
          .map_err(HumsafaarRouterError::Client)?;
        request.send(seedance_client).await
      }
    }
  }
}
