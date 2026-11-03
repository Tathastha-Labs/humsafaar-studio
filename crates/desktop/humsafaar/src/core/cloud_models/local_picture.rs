use tokens::tokens::media_files::MediaFileToken;

use crate::core::commands::enqueue::generate_error::GenerateError;
use crate::core::local_media::local_media_store::{mime_for_extension, LocalMediaStore};

/// A picture the user has on this computer, ready to hand to a cloud provider.
pub struct LocalPicture {
  pub bytes: Vec<u8>,
  pub mime: &'static str,
  pub file_name: String,
}

impl LocalPicture {
  /// Read the picture a media token points at. Pictures that live on a hosted account cannot be used.
  pub fn read(token: &MediaFileToken, local_media: &LocalMediaStore) -> Result<Self, GenerateError> {
    if !LocalMediaStore::is_local_token(token) {
      return Err(GenerateError::AnyhowError(anyhow::anyhow!(
        "That picture is not stored on this computer. Add it again from your computer."
      )));
    }
    let path = local_media.path_for_token(token)
      .ok_or_else(|| GenerateError::AnyhowError(anyhow::anyhow!("The picture could not be found")))?;
    let bytes = std::fs::read(&path)
      .map_err(|err| GenerateError::AnyhowError(anyhow::anyhow!("Could not read the picture: {}", err)))?;
    let extension = path.extension().and_then(|e| e.to_str());
    let file_name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "picture.png".to_string());
    Ok(Self { bytes, mime: mime_for_extension(extension), file_name })
  }

  /// An inline `data:` URI, which Fal accepts, so nothing has to be uploaded first.
  pub fn data_uri(&self) -> String {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    format!("data:{};base64,{}", self.mime, STANDARD.encode(&self.bytes))
  }
}
