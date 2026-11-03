use crate::api::router_provider::RouterProvider;
use crate::client::router_client::RouterClient;
use crate::errors::humsafaar_router_error::HumsafaarRouterError;
use crate::generate::generate_audio::audio_generation_cost_estimate::AudioGenerationCostEstimate;
use crate::generate::generate_audio::generate_audio_response::GenerateAudioResponse;
use crate::generate::generate_audio::providers::fal::seed_audio_1p0::cost::FalSeedAudio1p0CostState;
use crate::generate::generate_audio::providers::fal::seed_audio_1p0::request::FalSeedAudio1p0RequestState;
use crate::generate::generate_audio::providers::kinovi::suno_music::cost::KinoviSunoMusicCostState;
use crate::generate::generate_audio::providers::kinovi::suno_music::request::KinoviSunoMusicRequestState;
use crate::generate::generate_audio::providers::kinovi::suno_remix::cost::KinoviSunoRemixCostState;
use crate::generate::generate_audio::providers::kinovi::suno_remix::request::KinoviSunoRemixRequestState;
use crate::generate::generate_audio::providers::kinovi::suno_sample::cost::KinoviSunoSampleCostState;
use crate::generate::generate_audio::providers::kinovi::suno_sample::request::KinoviSunoSampleRequestState;
use crate::generate::generate_audio::providers::kinovi::suno_sounds::cost::KinoviSunoSoundsCostState;
use crate::generate::generate_audio::providers::kinovi::suno_sounds::request::KinoviSunoSoundsRequestState;

#[derive(Clone, Debug)]
pub enum AudioGenerationRequest {
  FalSeedAudio1p0(FalSeedAudio1p0RequestState),
  KinoviSunoMusic(KinoviSunoMusicRequestState),
  KinoviSunoRemix(KinoviSunoRemixRequestState),
  KinoviSunoSounds(KinoviSunoSoundsRequestState),
  KinoviSunoSample(KinoviSunoSampleRequestState),
}

impl AudioGenerationRequest {

  pub fn get_provider(&self) -> RouterProvider {
    match self {
      Self::FalSeedAudio1p0(_) => RouterProvider::Fal,
      Self::KinoviSunoMusic(_) => RouterProvider::KinoviWeb,
      Self::KinoviSunoRemix(_) => RouterProvider::KinoviWeb,
      Self::KinoviSunoSounds(_) => RouterProvider::KinoviWeb,
      Self::KinoviSunoSample(_) => RouterProvider::KinoviWeb,
    }
  }

  /// Return a cost estimate to fulfill the request.
  pub fn estimate_cost(&self) -> Result<AudioGenerationCostEstimate, HumsafaarRouterError> {
    match self {
      AudioGenerationRequest::FalSeedAudio1p0(request) => Ok(FalSeedAudio1p0CostState::from_request(request).estimate_cost()),
      AudioGenerationRequest::KinoviSunoMusic(request) => Ok(KinoviSunoMusicCostState::from_request(request).estimate_cost()),
      AudioGenerationRequest::KinoviSunoRemix(request) => Ok(KinoviSunoRemixCostState::from_request(request).estimate_cost()),
      AudioGenerationRequest::KinoviSunoSounds(request) => Ok(KinoviSunoSoundsCostState::from_request(request).estimate_cost()),
      AudioGenerationRequest::KinoviSunoSample(request) => Ok(KinoviSunoSampleCostState::from_request(request).estimate_cost()),
    }
  }

  /// Send the audio generation request
  /// If successful, returns the job IDs.
  pub async fn send_request(&self, client: &RouterClient) -> Result<GenerateAudioResponse, HumsafaarRouterError> {
    match self {
      AudioGenerationRequest::FalSeedAudio1p0(request) => {
        let client_ref = client.get_fal_client_ref()?;
        request.send(client_ref).await
      },
      AudioGenerationRequest::KinoviSunoMusic(request) => {
        let client_ref = client.get_kinovi_web_client_ref()?;
        request.send(client_ref).await
      },
      AudioGenerationRequest::KinoviSunoRemix(request) => {
        let client_ref = client.get_kinovi_web_client_ref()?;
        request.send(client_ref).await
      },
      AudioGenerationRequest::KinoviSunoSounds(request) => {
        let client_ref = client.get_kinovi_web_client_ref()?;
        request.send(client_ref).await
      },
      AudioGenerationRequest::KinoviSunoSample(request) => {
        let client_ref = client.get_kinovi_web_client_ref()?;
        request.send(client_ref).await
      },
    }
  }
}
