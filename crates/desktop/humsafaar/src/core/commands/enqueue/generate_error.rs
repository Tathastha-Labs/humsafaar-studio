use crate::core::humsafaar_error::HumsafaarError;
use anyhow::anyhow;
use humsafaar_router::errors::humsafaar_router_error::HumsafaarRouterError;
use humsafaar_router::errors::provider_error::ProviderError;
use base64::DecodeError;
use enums::common::generation_provider::GenerationProvider;
use enums::common::generation::common_model_type::CommonModelType;
use errors::AnyhowError;
use midjourney_client::error::midjourney_error::MidjourneyError;
use humsafaar_router::errors::download_error::DownloadError;
//use fal_client::error::fal_error_plus::FalErrorPlus;

#[derive(Debug)]
pub enum GenerateError {
  BadInput(BadInputReason),
  MissingCredentials(MissingCredentialsReason),
  ProviderFailure(ProviderFailureReason),

  /// We couldn't find a provider to dispatch the request to.
  NoProviderAvailable,
  
  /// We pulled out Fal (for now) - it's impacting build speeds. We'll add it in the future.
  FalNoLongerSupported,
  
  /// Wrong provider for the model
  BadProviderForModel {
    provider: GenerationProvider,
    model: CommonModelType,
  },

  /// If the response didn't contain job tokens to track.
  ResponseHadNoJobTokens,

  /// Issue with Humsafaar router downloads
  HumsafaarRouterDownloadError(DownloadError),

  /// A provider we don't support yet in Humsafaar Studio "the Tauri app" part.
  HumsafaarRouterNotYetSupportedProvider(&'static str),

  /// There was a billing, credits, or payments issue.
  BillingIssue(BillingIssueReason),

  /// The feature is not yet implemented.
  NotYetImplemented(String),

  // Misc error buckets
  AnyhowError(AnyhowError),
  DecodeError(DecodeError),
  IoError(std::io::Error),
}

#[derive(Debug)]
pub enum BadInputReason {
  Base64DecodeError,
  BothImageMaskMediaTokenAndBytesSupplied,
  CannotDetermineImageMimeType,
  InvalidNumberOfInputImages {
    provided: u32,
    min: u32,
    max: u32,
  },
  InvalidNumberOfRequestedImages {
    requested: u32,
    min: u32,
    max: u32,
  },
  NoModelSpecified,
  RequiredSourceImageMaskNotProvided,
  RequiredSourceImageNotProvided,
  WrongImageArguments(String),
}

#[derive(Debug)]
pub enum MissingCredentialsReason {
  NeedsGrokCredentials,
  NeedsFalApiKey,
  NeedsMidjourneyCredentials,
  NeedsMidjourneyUserId,
  NeedsMidjourneyUserInfo,
  NeedsSoraCredentials,
  NeedsWorldLabsCredentials,
}

#[derive(Debug)]
pub struct BillingIssueReason {
  pub provider: BillingProvider,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum BillingProvider {
  Humsafaar,
  Fal,
  Kinovi,
  Midjourney,
  Sora,
}

#[derive(Debug)]
pub enum ProviderFailureReason {
  //Fal(FalErrorPlus),
  MidjourneyError(MidjourneyError),
  /// NB: The midjourney client doesn't categorize all errors, so we have to do so on our end.
  MidjourneyJobEnqueueFailed,
}

impl GenerateError {

  //
  // Bad input
  //

  pub fn both_image_mask_media_token_and_bytes_supplied() -> Self {
    Self::BadInput(BadInputReason::BothImageMaskMediaTokenAndBytesSupplied)
  }

  pub fn no_model_specified() -> Self {
    Self::BadInput(BadInputReason::NoModelSpecified)
  }

  pub fn required_source_image_mask_not_provided() -> Self {
    Self::BadInput(BadInputReason::RequiredSourceImageMaskNotProvided)
  }

  pub fn required_source_image_not_provided() -> Self {
    Self::BadInput(BadInputReason::RequiredSourceImageNotProvided)
  }

  //
  // Missing credentials
  //

  pub fn needs_fal_api_key() -> Self {
    Self::MissingCredentials(MissingCredentialsReason::NeedsFalApiKey)
  }

  pub fn needs_grok_credentials() -> Self {
    Self::MissingCredentials(MissingCredentialsReason::NeedsGrokCredentials)
  }

  pub fn needs_midjourney_credentials() -> Self {
    Self::MissingCredentials(MissingCredentialsReason::NeedsMidjourneyCredentials)
  }

  pub fn needs_sora_credentials() -> Self {
    Self::MissingCredentials(MissingCredentialsReason::NeedsSoraCredentials)
  }

  pub fn needs_worldlabs_credentials() -> Self {
    Self::MissingCredentials(MissingCredentialsReason::NeedsWorldLabsCredentials)
  }
}

impl From<AnyhowError> for GenerateError {
  fn from(value: AnyhowError) -> Self {
    Self::AnyhowError(value)
  }
}

impl From<HumsafaarError> for GenerateError {
  fn from(value: HumsafaarError) -> Self {
    match value {
      HumsafaarError::AnyhowError(e) => Self::AnyhowError(e),
      HumsafaarError::DecodeError(e) => Self::DecodeError(e),
      HumsafaarError::IoError(e) => Self::IoError(e),
      HumsafaarError::RwLockReadError => Self::AnyhowError(anyhow!("Lock read error")),
      HumsafaarError::RwLockWriteError => Self::AnyhowError(anyhow!("Lock write error")),
      HumsafaarError::MutexLockError => Self::AnyhowError(anyhow!("Mutex lock error")),
      HumsafaarError::ReqwestError(err) => Self::AnyhowError(anyhow!("Reqwest error: {:?}", err)),
      HumsafaarError::BadDownloadFilename { path } => Self::AnyhowError(anyhow!("Bad download file name: {:?}", path)),
      HumsafaarError::CannotDownloadFilePathAlreadyExists { path } => Self::AnyhowError(anyhow!("Cannot download already existing file: {:?}", path)),
    }
  }
}

//impl From<FalErrorPlus> for GenerateError {
//  fn from(value: FalErrorPlus) -> Self {
//    Self::ProviderFailure(ProviderFailureReason::Fal(value))
//  }
//}

impl From<MidjourneyError> for GenerateError {
  fn from(value: MidjourneyError) -> Self {
    Self::ProviderFailure(ProviderFailureReason::MidjourneyError(value))
  }
}

impl From<HumsafaarRouterError> for GenerateError {
  fn from(value: HumsafaarRouterError) -> Self {
    match value {
      HumsafaarRouterError::Client(e) => Self::BadInput(BadInputReason::WrongImageArguments(e.to_string())),
      HumsafaarRouterError::Download(e) => Self::HumsafaarRouterDownloadError(e),
      HumsafaarRouterError::InvalidInput(msg) => Self::BadInput(BadInputReason::WrongImageArguments(msg)),
      HumsafaarRouterError::ProviderBillingError(err) => {
        let provider = match err {
          ProviderError::Midjourney(_) | ProviderError::MidjourneySubscriptionRequired(_) | ProviderError::MidjourneySubmitRejected(_) => BillingProvider::Midjourney,
          ProviderError::Fal(_) => BillingProvider::Fal,
          ProviderError::KinoviWeb(_) => BillingProvider::Kinovi,
          ProviderError::GmiCloud(_) => BillingProvider::Humsafaar,
          ProviderError::Grok(_) => BillingProvider::Humsafaar,
          ProviderError::WorldLabs(_) => BillingProvider::Humsafaar,
        };
        Self::BillingIssue(BillingIssueReason { provider })
      },
      HumsafaarRouterError::Provider(ProviderError::Midjourney(e)) => Self::from(e),
      HumsafaarRouterError::Provider(ProviderError::MidjourneySubscriptionRequired(message) | ProviderError::MidjourneySubmitRejected(message)) => Self::BadInput(BadInputReason::WrongImageArguments(message)),
      HumsafaarRouterError::Provider(ProviderError::Fal(_)) => Self::FalNoLongerSupported,
      HumsafaarRouterError::Provider(ProviderError::GmiCloud(_)) => Self::HumsafaarRouterNotYetSupportedProvider("gmicloud"),
      HumsafaarRouterError::Provider(ProviderError::Grok(_)) => Self::HumsafaarRouterNotYetSupportedProvider("grok_api"),
      HumsafaarRouterError::Provider(ProviderError::KinoviWeb(_)) => Self::HumsafaarRouterNotYetSupportedProvider("kinovi_web"),
      // NB: the desktop app reaches World Labs through the Humsafaar backend; the router's
      // direct World Labs provider isn't used here.
      HumsafaarRouterError::Provider(ProviderError::WorldLabs(_)) => Self::HumsafaarRouterNotYetSupportedProvider("world_labs"),
      HumsafaarRouterError::UnsupportedModel(model) => Self::NotYetImplemented(format!("Unsupported model: {}", model)),
      HumsafaarRouterError::UnsupportedProviderAndModelForNewApi(message) => Self::HumsafaarRouterNotYetSupportedProvider("unsupported model for new router API"),
    }
  }
}
