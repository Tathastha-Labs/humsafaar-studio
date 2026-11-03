use crate::api::router_provider::RouterProvider;
use crate::client::router_client::RouterClient;
use crate::errors::humsafaar_router_error::HumsafaarRouterError;
use crate::generate::generate_video::generate_video_response::GenerateVideoResponse;
use crate::generate::generate_video::video_generation_cost_estimate::VideoGenerationCostEstimate;
use crate::generate::generate_video::providers::fal::kling_1_6_pro::cost::FalKling16ProCostState;
use crate::generate::generate_video::providers::fal::kling_1_6_pro::request::FalKling16ProRequestState;
use crate::generate::generate_video::providers::fal::kling_2_1_master::cost::FalKling21MasterCostState;
use crate::generate::generate_video::providers::fal::kling_2_1_master::request::FalKling21MasterRequestState;
use crate::generate::generate_video::providers::fal::kling_2_1_pro::cost::FalKling21ProCostState;
use crate::generate::generate_video::providers::fal::kling_2_1_pro::request::FalKling21ProRequestState;
use crate::generate::generate_video::providers::fal::kling_2_5_turbo_pro::cost::FalKling2p5TurboProCostState;
use crate::generate::generate_video::providers::fal::kling_2_5_turbo_pro::request::FalKling2p5TurboProRequestState;
use crate::generate::generate_video::providers::fal::kling_2_6_pro::cost::FalKling2p6ProCostState;
use crate::generate::generate_video::providers::fal::kling_2_6_pro::request::FalKling2p6ProRequestState;
use crate::generate::generate_video::providers::fal::kling_3p0_pro::cost::FalKling3p0ProCostState;
use crate::generate::generate_video::providers::fal::kling_3p0_pro::request::FalKling3p0ProRequestState;
use crate::generate::generate_video::providers::fal::flux_3::cost::FalFlux3CostState;
use crate::generate::generate_video::providers::fal::flux_3::request::FalFlux3RequestState;
use crate::generate::generate_video::providers::fal::flux_3_draft::cost::FalFlux3DraftCostState;
use crate::generate::generate_video::providers::fal::flux_3_draft::request::FalFlux3DraftRequestState;
use crate::generate::generate_video::providers::fal::kling_3p0_standard::cost::FalKling3p0StandardCostState;
use crate::generate::generate_video::providers::fal::kling_3p0_standard::request::FalKling3p0StandardRequestState;
use crate::generate::generate_video::providers::fal::minimax_h3::cost::FalMinimaxH3CostState;
use crate::generate::generate_video::providers::fal::minimax_h3::request::FalMinimaxH3RequestState;
use crate::generate::generate_video::providers::fal::seedance_1p0_lite::cost::FalSeedance10LiteCostState;
use crate::generate::generate_video::providers::fal::seedance_1p0_lite::request::FalSeedance10LiteRequestState;
use crate::generate::generate_video::providers::fal::sora_2::cost::FalSora2CostState;
use crate::generate::generate_video::providers::fal::sora_2::request::FalSora2RequestState;
use crate::generate::generate_video::providers::fal::sora_2_pro::cost::FalSora2ProCostState;
use crate::generate::generate_video::providers::fal::sora_2_pro::request::FalSora2ProRequestState;
use crate::generate::generate_video::providers::fal::seedance_1p5_pro::cost::FalSeedance1p5ProCostState;
use crate::generate::generate_video::providers::fal::seedance_1p5_pro::request::FalSeedance1p5ProRequestState;
use crate::generate::generate_video::providers::fal::veo_2::cost::FalVeo2CostState;
use crate::generate::generate_video::providers::fal::veo_2::request::FalVeo2RequestState;
use crate::generate::generate_video::providers::fal::veo_3::cost::FalVeo3CostState;
use crate::generate::generate_video::providers::fal::veo_3::request::FalVeo3RequestState;
use crate::generate::generate_video::providers::fal::veo_3_fast::cost::FalVeo3FastCostState;
use crate::generate::generate_video::providers::fal::veo_3_fast::request::FalVeo3FastRequestState;
use crate::generate::generate_video::providers::fal::veo_3p1::cost::FalVeo3p1CostState;
use crate::generate::generate_video::providers::fal::veo_3p1::request::FalVeo3p1RequestState;
use crate::generate::generate_video::providers::fal::veo_3p1_fast::cost::FalVeo3p1FastCostState;
use crate::generate::generate_video::providers::fal::veo_3p1_fast::request::FalVeo3p1FastRequestState;
use crate::generate::generate_video::providers::fal::veo_3p1_lite::cost::FalVeo3p1LiteCostState;
use crate::generate::generate_video::providers::fal::veo_3p1_lite::request::FalVeo3p1LiteRequestState;
use crate::generate::generate_video::providers::fal::vidu_q3::cost::FalViduQ3CostState;
use crate::generate::generate_video::providers::fal::vidu_q3::request::FalViduQ3RequestState;
use crate::generate::generate_video::providers::fal::vidu_q3_turbo::cost::FalViduQ3TurboCostState;
use crate::generate::generate_video::providers::fal::vidu_q3_turbo::request::FalViduQ3TurboRequestState;
use crate::generate::generate_video::providers::grok_api::grok_imagine_video::cost::GrokApiGrokImagineVideoCostState;
use crate::generate::generate_video::providers::grok_api::grok_imagine_video::request::GrokApiGrokImagineVideoRequestState;
use crate::generate::generate_video::providers::grok_api::grok_imagine_video_1p5::cost::GrokApiGrokImagineVideo1p5CostState;
use crate::generate::generate_video::providers::grok_api::grok_imagine_video_1p5::request::GrokApiGrokImagineVideo1p5RequestState;
use crate::generate::generate_video::providers::kinovi::happy_horse_1p0::cost::KinoviHappyHorse1p0CostState;
use crate::generate::generate_video::providers::kinovi::happy_horse_1p0::request::KinoviHappyHorse1p0RequestState;
use crate::generate::generate_video::providers::kinovi::seedance_2p0::cost::KinoviSeedance2p0CostState;
use crate::generate::generate_video::providers::kinovi::seedance_2p0::request::KinoviSeedance2p0RequestState;
use crate::generate::generate_video::providers::kinovi::seedance_2p0_fast::cost::KinoviSeedance2p0FastCostState;
use crate::generate::generate_video::providers::kinovi::seedance_2p0_fast::request::KinoviSeedance2p0FastRequestState;
use crate::generate::generate_video::providers::kinovi::seedance_2p0_mini::cost::KinoviSeedance2p0MiniCostState;
use crate::generate::generate_video::providers::kinovi::seedance_2p0_mini::request::KinoviSeedance2p0MiniRequestState;
use crate::generate::generate_video::providers::kinovi::seedance_2p5::cost::KinoviSeedance2p5CostState;
use crate::generate::generate_video::providers::kinovi::seedance_2p5::request::KinoviSeedance2p5RequestState;
use crate::generate::generate_video::providers::kinovi::seedance_2p5_preview::cost::KinoviSeedance2p5PreviewCostState;
use crate::generate::generate_video::providers::kinovi::seedance_2p5_preview::request::KinoviSeedance2p5PreviewRequestState;

#[derive(Clone, Debug)]
pub enum VideoGenerationRequest {
  FalFlux3(FalFlux3RequestState),
  FalFlux3Draft(FalFlux3DraftRequestState),
  FalKling16Pro(FalKling16ProRequestState),
  FalKling21Master(FalKling21MasterRequestState),
  FalKling21Pro(FalKling21ProRequestState),
  FalKling2p5TurboPro(FalKling2p5TurboProRequestState),
  FalKling2p6Pro(FalKling2p6ProRequestState),
  FalKling3p0Pro(FalKling3p0ProRequestState),
  FalKling3p0Standard(FalKling3p0StandardRequestState),
  FalMinimaxH3(FalMinimaxH3RequestState),
  FalSeedance10Lite(FalSeedance10LiteRequestState),
  FalSeedance1p5Pro(FalSeedance1p5ProRequestState),
  FalSora2(FalSora2RequestState),
  FalSora2Pro(FalSora2ProRequestState),
  FalVeo2(FalVeo2RequestState),
  FalVeo3(FalVeo3RequestState),
  FalVeo3Fast(FalVeo3FastRequestState),
  FalVeo3p1(FalVeo3p1RequestState),
  FalVeo3p1Fast(FalVeo3p1FastRequestState),
  FalVeo3p1Lite(FalVeo3p1LiteRequestState),
  FalViduQ3(FalViduQ3RequestState),
  FalViduQ3Turbo(FalViduQ3TurboRequestState),
  GrokApiGrokImagineVideo(GrokApiGrokImagineVideoRequestState),
  GrokApiGrokImagineVideo1p5(GrokApiGrokImagineVideo1p5RequestState),
  KinoviHappyHorse1p0(KinoviHappyHorse1p0RequestState),
  KinoviSeedance2p0(KinoviSeedance2p0RequestState),
  KinoviSeedance2p0Fast(KinoviSeedance2p0FastRequestState),
  KinoviSeedance2p0Mini(KinoviSeedance2p0MiniRequestState),
  KinoviSeedance2p5Preview(KinoviSeedance2p5PreviewRequestState),
  KinoviSeedance2p5(KinoviSeedance2p5RequestState),
}

impl VideoGenerationRequest {

  pub fn get_provider(&self) -> RouterProvider {
    match self {
      Self::FalKling16Pro(_) => RouterProvider::Fal,
      Self::FalKling21Master(_) => RouterProvider::Fal,
      Self::FalKling21Pro(_) => RouterProvider::Fal,
      Self::FalKling2p5TurboPro(_) => RouterProvider::Fal,
      Self::FalKling2p6Pro(_) => RouterProvider::Fal,
      Self::FalKling3p0Pro(_) => RouterProvider::Fal,
      Self::FalFlux3(_) => RouterProvider::Fal,
      Self::FalFlux3Draft(_) => RouterProvider::Fal,
      Self::FalKling3p0Standard(_) => RouterProvider::Fal,
      Self::FalMinimaxH3(_) => RouterProvider::Fal,
      Self::FalSeedance10Lite(_) => RouterProvider::Fal,
      Self::FalSeedance1p5Pro(_) => RouterProvider::Fal,
      Self::FalSora2(_) => RouterProvider::Fal,
      Self::FalSora2Pro(_) => RouterProvider::Fal,
      Self::FalVeo2(_) => RouterProvider::Fal,
      Self::FalVeo3(_) => RouterProvider::Fal,
      Self::FalVeo3Fast(_) => RouterProvider::Fal,
      Self::FalVeo3p1(_) => RouterProvider::Fal,
      Self::FalVeo3p1Fast(_) => RouterProvider::Fal,
      Self::FalVeo3p1Lite(_) => RouterProvider::Fal,
      Self::FalViduQ3(_) => RouterProvider::Fal,
      Self::FalViduQ3Turbo(_) => RouterProvider::Fal,
      Self::GrokApiGrokImagineVideo(_) => RouterProvider::GrokApi,
      Self::GrokApiGrokImagineVideo1p5(_) => RouterProvider::GrokApi,
      Self::KinoviHappyHorse1p0(_) => RouterProvider::KinoviWeb,
      Self::KinoviSeedance2p0(_) => RouterProvider::KinoviWeb,
      Self::KinoviSeedance2p0Fast(_) => RouterProvider::KinoviWeb,
      Self::KinoviSeedance2p0Mini(_) => RouterProvider::KinoviWeb,
      Self::KinoviSeedance2p5Preview(_) => RouterProvider::KinoviWeb,
      Self::KinoviSeedance2p5(_) => RouterProvider::KinoviWeb,
    }
  }

  /// Return a cost estimate to fulfill the request.
  pub fn estimate_cost(&self) -> Result<VideoGenerationCostEstimate, HumsafaarRouterError> {
    match self {
      VideoGenerationRequest::FalKling16Pro(request) => Ok(FalKling16ProCostState::from_request(request).estimate_cost()),
      VideoGenerationRequest::FalKling21Master(request) => Ok(FalKling21MasterCostState::from_request(request).estimate_cost()),
      VideoGenerationRequest::FalKling21Pro(request) => Ok(FalKling21ProCostState::from_request(request).estimate_cost()),
      VideoGenerationRequest::FalKling2p5TurboPro(request) => Ok(FalKling2p5TurboProCostState::from_request(request).estimate_cost()),
      VideoGenerationRequest::FalKling2p6Pro(request) => Ok(FalKling2p6ProCostState::from_request(request).estimate_cost()),
      VideoGenerationRequest::FalKling3p0Pro(request) => Ok(FalKling3p0ProCostState::from_request(request).estimate_cost()),
      VideoGenerationRequest::FalFlux3(request) => Ok(FalFlux3CostState::from_request(request).estimate_cost()),
      VideoGenerationRequest::FalFlux3Draft(request) => Ok(FalFlux3DraftCostState::from_request(request).estimate_cost()),
      VideoGenerationRequest::FalKling3p0Standard(request) => Ok(FalKling3p0StandardCostState::from_request(request).estimate_cost()),
      VideoGenerationRequest::FalMinimaxH3(request) => Ok(FalMinimaxH3CostState::from_request(request).estimate_cost()),
      VideoGenerationRequest::FalSeedance10Lite(request) => Ok(FalSeedance10LiteCostState::from_request(request).estimate_cost()),
      VideoGenerationRequest::FalSeedance1p5Pro(request) => Ok(FalSeedance1p5ProCostState::from_request(request).estimate_cost()),
      VideoGenerationRequest::FalSora2(request) => Ok(FalSora2CostState::from_request(request).estimate_cost()),
      VideoGenerationRequest::FalSora2Pro(request) => Ok(FalSora2ProCostState::from_request(request).estimate_cost()),
      VideoGenerationRequest::FalVeo2(request) => Ok(FalVeo2CostState::from_request(request).estimate_cost()),
      VideoGenerationRequest::FalVeo3(request) => Ok(FalVeo3CostState::from_request(request).estimate_cost()),
      VideoGenerationRequest::FalVeo3Fast(request) => Ok(FalVeo3FastCostState::from_request(request).estimate_cost()),
      VideoGenerationRequest::FalVeo3p1(request) => Ok(FalVeo3p1CostState::from_request(request).estimate_cost()),
      VideoGenerationRequest::FalVeo3p1Fast(request) => Ok(FalVeo3p1FastCostState::from_request(request).estimate_cost()),
      VideoGenerationRequest::FalVeo3p1Lite(request) => Ok(FalVeo3p1LiteCostState::from_request(request).estimate_cost()),
      VideoGenerationRequest::FalViduQ3(request) => Ok(FalViduQ3CostState::from_request(request).estimate_cost()),
      VideoGenerationRequest::FalViduQ3Turbo(request) => Ok(FalViduQ3TurboCostState::from_request(request).estimate_cost()),
      VideoGenerationRequest::GrokApiGrokImagineVideo(request) => Ok(GrokApiGrokImagineVideoCostState::from_request(request).estimate_cost()),
      VideoGenerationRequest::GrokApiGrokImagineVideo1p5(request) => GrokApiGrokImagineVideo1p5CostState::from_request(request).estimate_cost(),
      VideoGenerationRequest::KinoviHappyHorse1p0(request) => Ok(KinoviHappyHorse1p0CostState::from_request(request).estimate_cost()),
      VideoGenerationRequest::KinoviSeedance2p0(request) => Ok(KinoviSeedance2p0CostState::from_request(request).estimate_cost()),
      VideoGenerationRequest::KinoviSeedance2p0Fast(request) => Ok(KinoviSeedance2p0FastCostState::from_request(request).estimate_cost()),
      VideoGenerationRequest::KinoviSeedance2p0Mini(request) => Ok(KinoviSeedance2p0MiniCostState::from_request(request).estimate_cost()),
      VideoGenerationRequest::KinoviSeedance2p5Preview(request) => Ok(KinoviSeedance2p5PreviewCostState::from_request(request).estimate_cost()),
      VideoGenerationRequest::KinoviSeedance2p5(request) => Ok(KinoviSeedance2p5CostState::from_request(request).estimate_cost()),
    }
  }

  /// Send the video generation request
  /// If successful, returns the job IDs.
  pub async fn send_request(&self, client: &RouterClient) -> Result<GenerateVideoResponse, HumsafaarRouterError> {
    match self {
      VideoGenerationRequest::FalKling16Pro(request) => {
        let client_ref = client.get_fal_client_ref()?;
        request.send(client_ref).await
      },
      VideoGenerationRequest::FalKling21Master(request) => {
        let client_ref = client.get_fal_client_ref()?;
        request.send(client_ref).await
      },
      VideoGenerationRequest::FalKling21Pro(request) => {
        let client_ref = client.get_fal_client_ref()?;
        request.send(client_ref).await
      },
      VideoGenerationRequest::FalKling2p5TurboPro(request) => {
        let client_ref = client.get_fal_client_ref()?;
        request.send(client_ref).await
      },
      VideoGenerationRequest::FalKling2p6Pro(request) => {
        let client_ref = client.get_fal_client_ref()?;
        request.send(client_ref).await
      },
      VideoGenerationRequest::FalKling3p0Pro(request) => {
        let client_ref = client.get_fal_client_ref()?;
        request.send(client_ref).await
      },
      VideoGenerationRequest::FalKling3p0Standard(request) => {
        let client_ref = client.get_fal_client_ref()?;
        request.send(client_ref).await
      },
      VideoGenerationRequest::FalMinimaxH3(request) => {
        let client_ref = client.get_fal_client_ref()?;
        request.send(client_ref).await
      },
      VideoGenerationRequest::FalFlux3(request) => {
        let client_ref = client.get_fal_client_ref()?;
        request.send(client_ref).await
      },
      VideoGenerationRequest::FalFlux3Draft(request) => {
        let client_ref = client.get_fal_client_ref()?;
        request.send(client_ref).await
      },
      VideoGenerationRequest::FalSeedance10Lite(request) => {
        let client_ref = client.get_fal_client_ref()?;
        request.send(client_ref).await
      },
      VideoGenerationRequest::FalSeedance1p5Pro(request) => {
        let client_ref = client.get_fal_client_ref()?;
        request.send(client_ref).await
      },
      VideoGenerationRequest::FalSora2(request) => {
        let client_ref = client.get_fal_client_ref()?;
        request.send(client_ref).await
      },
      VideoGenerationRequest::FalSora2Pro(request) => {
        let client_ref = client.get_fal_client_ref()?;
        request.send(client_ref).await
      },
      VideoGenerationRequest::FalVeo2(request) => {
        let client_ref = client.get_fal_client_ref()?;
        request.send(client_ref).await
      },
      VideoGenerationRequest::FalVeo3(request) => {
        let client_ref = client.get_fal_client_ref()?;
        request.send(client_ref).await
      },
      VideoGenerationRequest::FalVeo3Fast(request) => {
        let client_ref = client.get_fal_client_ref()?;
        request.send(client_ref).await
      },
      VideoGenerationRequest::FalVeo3p1(request) => {
        let client_ref = client.get_fal_client_ref()?;
        request.send(client_ref).await
      },
      VideoGenerationRequest::FalVeo3p1Fast(request) => {
        let client_ref = client.get_fal_client_ref()?;
        request.send(client_ref).await
      },
      VideoGenerationRequest::FalVeo3p1Lite(request) => {
        let client_ref = client.get_fal_client_ref()?;
        request.send(client_ref).await
      },
      VideoGenerationRequest::FalViduQ3(request) => {
        let client_ref = client.get_fal_client_ref()?;
        request.send(client_ref).await
      },
      VideoGenerationRequest::FalViduQ3Turbo(request) => {
        let client_ref = client.get_fal_client_ref()?;
        request.send(client_ref).await
      },
      VideoGenerationRequest::GrokApiGrokImagineVideo(request) => {
        let client_ref = client.get_grok_api_client_ref()?;
        request.send(client_ref).await
      },
      VideoGenerationRequest::GrokApiGrokImagineVideo1p5(request) => {
        let client_ref = client.get_grok_api_client_ref()?;
        request.send(client_ref).await
      },
      VideoGenerationRequest::KinoviHappyHorse1p0(request) => {
        let client_ref = client.get_kinovi_web_client_ref()?;
        request.send(client_ref).await
      },
      VideoGenerationRequest::KinoviSeedance2p0(request) => {
        let client_ref = client.get_kinovi_web_client_ref()?;
        request.send(client_ref).await
      },
      VideoGenerationRequest::KinoviSeedance2p0Fast(request) => {
        let client_ref = client.get_kinovi_web_client_ref()?;
        request.send(client_ref).await
      },
      VideoGenerationRequest::KinoviSeedance2p0Mini(request) => {
        let client_ref = client.get_kinovi_web_client_ref()?;
        request.send(client_ref).await
      },
      VideoGenerationRequest::KinoviSeedance2p5Preview(request) => {
        let client_ref = client.get_kinovi_web_client_ref()?;
        request.send(client_ref).await
      },
      VideoGenerationRequest::KinoviSeedance2p5(request) => {
        let client_ref = client.get_kinovi_web_client_ref()?;
        request.send(client_ref).await
      },
    }
  }
}
