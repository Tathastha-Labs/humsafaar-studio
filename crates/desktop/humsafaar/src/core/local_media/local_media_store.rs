use crate::core::state::data_dir::app_data_root::AppDataRoot;
use crate::core::state::data_dir::trait_data_subdir::DataSubdir;
use anyhow::{anyhow, Result as AnyhowResult};
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use percent_encoding::{utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};
use std::fs;
use std::path::{Path, PathBuf};
use tokens::tokens::media_files::MediaFileToken;
use uuid_utils::uuid::generate_random_uuid;

/// Tokens minted by this store start with this prefix. Hosted tokens never do,
/// so it is safe to branch on it when a token could be either kind.
pub const LOCAL_MEDIA_TOKEN_PREFIX: &str = "mf_local_";

/// The character after the prefix says where a file came from. The rest of a token is hex, so
/// neither letter can be confused with it, and tokens made before this marker count as generated.
const GENERATED_MARKER: char = 'g';
const UPLOAD_MARKER: char = 'u';

/// Where a stored file came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MediaOrigin {
  /// Made by a model, here or in the cloud.
  Generated,
  /// Added by the user, or exported from an editor.
  Upload,
}

/// One file in the store.
#[derive(Clone, Debug)]
pub struct StoredMedia {
  pub token: MediaFileToken,
  pub path: PathBuf,
  pub extension: String,
  pub origin: MediaOrigin,
  pub modified: std::time::SystemTime,
  pub size_bytes: u64,
}

/// Mirrors JavaScript's `encodeURIComponent`, which is what Tauri's
/// `convertFileSrc` uses to build asset protocol URLs.
const URI_COMPONENT: &AsciiSet = &NON_ALPHANUMERIC
  .remove(b'-').remove(b'_').remove(b'.').remove(b'!')
  .remove(b'~').remove(b'*').remove(b'\'').remove(b'(').remove(b')');

/// Files saved on the local disk, keyed by a token, as `<root>/<token>.<ext>`.
/// There is no index: the directory listing is the source of truth.
#[derive(Clone)]
pub struct LocalMediaStore {
  root: PathBuf,
}

impl LocalMediaStore {
  pub fn new<P: AsRef<Path>>(root: P) -> Self {
    Self { root: root.as_ref().to_path_buf() }
  }

  pub fn from_app_data_root(app_data_root: &AppDataRoot) -> Self {
    Self::new(app_data_root.media_dir().path())
  }

  pub fn is_local_token(token: &MediaFileToken) -> bool {
    token.as_str().starts_with(LOCAL_MEDIA_TOKEN_PREFIX)
  }

  /// Copy an existing file into the store. Used for results, so they count as generated.
  pub fn put_file(&self, source: &Path) -> AnyhowResult<MediaFileToken> {
    let extension = source.extension().and_then(|e| e.to_str());
    let (token, destination) = self.new_destination(extension, MediaOrigin::Generated)?;
    fs::copy(source, &destination)?;
    Ok(token)
  }

  /// Save bytes the user brought, so they count as uploads.
  pub fn put_bytes(&self, bytes: &[u8], extension: &str) -> AnyhowResult<MediaFileToken> {
    let (token, destination) = self.new_destination(Some(extension), MediaOrigin::Upload)?;
    fs::write(&destination, bytes)?;
    Ok(token)
  }

  /// Find the file for a token. `None` if the token is not local or the file is gone.
  pub fn path_for_token(&self, token: &MediaFileToken) -> Option<PathBuf> {
    if !Self::is_local_token(token) || !is_safe_token(token.as_str()) {
      return None;
    }
    let prefix = format!("{}.", token.as_str());
    fs::read_dir(&self.root).ok()?
      .filter_map(|entry| entry.ok())
      .find(|entry| entry.file_name().to_string_lossy().starts_with(&prefix))
      .map(|entry| entry.path())
  }

  /// Replace the contents of an existing file, keeping its token. Used to save a scene again.
  pub fn replace_bytes(&self, token: &MediaFileToken, bytes: &[u8]) -> AnyhowResult<()> {
    let path = self.path_for_token(token)
      .ok_or_else(|| anyhow!("local media file not found: {}", token.as_str()))?;
    // Write beside the file and rename over it, so a crash cannot leave half a scene.
    let temporary = path.with_extension("part");
    fs::write(&temporary, bytes)?;
    fs::rename(&temporary, &path)?;
    Ok(())
  }

  /// A file's contents as text. Fails if it is not valid UTF-8.
  pub fn read_text(&self, token: &MediaFileToken) -> AnyhowResult<String> {
    let path = self.path_for_token(token)
      .ok_or_else(|| anyhow!("local media file not found: {}", token.as_str()))?;
    Ok(fs::read_to_string(path)?)
  }

  /// Where a local token's file came from.
  pub fn origin_of(token: &MediaFileToken) -> MediaOrigin {
    let rest = token.as_str().strip_prefix(LOCAL_MEDIA_TOKEN_PREFIX).unwrap_or("");
    if rest.starts_with(UPLOAD_MARKER) { MediaOrigin::Upload } else { MediaOrigin::Generated }
  }

  /// Every file in the store, newest first.
  pub fn list(&self) -> Vec<StoredMedia> {
    let Ok(entries) = fs::read_dir(&self.root) else {
      return Vec::new();
    };
    let mut files: Vec<StoredMedia> = entries
      .filter_map(|entry| entry.ok())
      .filter_map(|entry| {
        let path = entry.path();
        let stem = path.file_stem()?.to_str()?.to_string();
        if !stem.starts_with(LOCAL_MEDIA_TOKEN_PREFIX) || !is_safe_token(&stem) {
          return None;
        }
        let metadata = entry.metadata().ok()?;
        let token = MediaFileToken::new_from_str(&stem);
        Some(StoredMedia {
          origin: Self::origin_of(&token),
          extension: path.extension()?.to_str()?.to_ascii_lowercase(),
          modified: metadata.modified().ok()?,
          size_bytes: metadata.len(),
          token,
          path,
        })
      })
      .collect();
    files.sort_by(|a, b| b.modified.cmp(&a.modified));
    files
  }

  /// Remove the file for a token. Returns whether a file was deleted.
  pub fn delete(&self, token: &MediaFileToken) -> AnyhowResult<bool> {
    match self.path_for_token(token) {
      Some(path) => {
        fs::remove_file(path)?;
        Ok(true)
      }
      None => Ok(false),
    }
  }

  /// A `data:` URI of the file, for providers that accept inline image inputs.
  pub fn data_uri_for_token(&self, token: &MediaFileToken) -> AnyhowResult<String> {
    let path = self.path_for_token(token)
      .ok_or_else(|| anyhow!("local media file not found: {}", token.as_str()))?;
    let bytes = fs::read(&path)?;
    let mime = mime_for_extension(path.extension().and_then(|e| e.to_str()));
    Ok(format!("data:{};base64,{}", mime, BASE64_STANDARD.encode(bytes)))
  }

  /// A URL the webview can load, via Tauri's asset protocol (enabled with scope `**`).
  pub fn asset_url_for_path(path: &Path) -> String {
    let encoded = utf8_percent_encode(&path.to_string_lossy(), URI_COMPONENT).to_string();
    if cfg!(windows) {
      format!("http://asset.localhost/{}", encoded)
    } else {
      format!("asset://localhost/{}", encoded)
    }
  }

  fn new_destination(&self, extension: Option<&str>, origin: MediaOrigin) -> AnyhowResult<(MediaFileToken, PathBuf)> {
    fs::create_dir_all(&self.root)?;
    let marker = match origin {
      MediaOrigin::Generated => GENERATED_MARKER,
      MediaOrigin::Upload => UPLOAD_MARKER,
    };
    let token = MediaFileToken::new_from_str(
      &format!("{}{}{}", LOCAL_MEDIA_TOKEN_PREFIX, marker, generate_random_uuid().replace('-', "")),
    );
    let extension = sanitize_extension(extension);
    let destination = self.root.join(format!("{}.{}", token.as_str(), extension));
    Ok((token, destination))
  }
}

/// Tokens come from the frontend, so refuse anything that could escape the directory.
fn is_safe_token(token: &str) -> bool {
  token.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn sanitize_extension(extension: Option<&str>) -> String {
  let cleaned: String = extension.unwrap_or("")
    .trim_start_matches('.')
    .chars()
    .filter(|c| c.is_ascii_alphanumeric())
    .take(8)
    .collect::<String>()
    .to_lowercase();
  if cleaned.is_empty() { "bin".to_string() } else { cleaned }
}

pub fn mime_for_extension(extension: Option<&str>) -> &'static str {
  match extension.map(|e| e.to_ascii_lowercase()).as_deref() {
    Some("png") => "image/png",
    Some("jpg") | Some("jpeg") => "image/jpeg",
    Some("webp") => "image/webp",
    Some("gif") => "image/gif",
    Some("mp4") => "video/mp4",
    Some("webm") => "video/webm",
    Some("glb") => "model/gltf-binary",
    _ => "application/octet-stream",
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use tempfile::tempdir;

  #[test]
  fn put_bytes_roundtrips_and_token_is_local() {
    let dir = tempdir().unwrap();
    let store = LocalMediaStore::new(dir.path());
    let token = store.put_bytes(b"hello", "PNG").unwrap();

    assert!(LocalMediaStore::is_local_token(&token));
    let path = store.path_for_token(&token).unwrap();
    assert_eq!(path.extension().unwrap(), "png");
    assert_eq!(fs::read(path).unwrap(), b"hello");
  }

  #[test]
  fn uploads_and_results_are_told_apart() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("in.png");
    fs::write(&source, b"abc").unwrap();
    let store = LocalMediaStore::new(dir.path().join("media"));
    let uploaded = store.put_bytes(b"x", "png").unwrap();
    let generated = store.put_file(&source).unwrap();

    assert_eq!(LocalMediaStore::origin_of(&uploaded), MediaOrigin::Upload);
    assert_eq!(LocalMediaStore::origin_of(&generated), MediaOrigin::Generated);
    // A token from before the marker existed counts as generated.
    assert_eq!(LocalMediaStore::origin_of(&MediaFileToken::new_from_str("mf_local_0a1b2c")), MediaOrigin::Generated);
  }

  #[test]
  fn list_shows_every_file_and_ignores_strangers() {
    let dir = tempdir().unwrap();
    let store = LocalMediaStore::new(dir.path());
    let first = store.put_bytes(b"1", "png").unwrap();
    std::fs::write(dir.path().join("notes.txt"), b"x").unwrap();
    std::fs::write(dir.path().join("mf_local_../evil.png"), b"x").ok();
    let listed = store.list();

    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].token, first);
    assert_eq!(listed[0].extension, "png");
    assert_eq!(listed[0].origin, MediaOrigin::Upload);
    assert_eq!(listed[0].size_bytes, 1);
  }

  #[test]
  fn replacing_keeps_the_token_and_changes_the_contents() {
    let dir = tempdir().unwrap();
    let store = LocalMediaStore::new(dir.path());
    let token = store.put_bytes(b"{\"v\":1}", "scene").unwrap();
    store.replace_bytes(&token, b"{\"v\":2}").unwrap();

    assert_eq!(store.read_text(&token).unwrap(), "{\"v\":2}");
    assert_eq!(store.list().len(), 1);
    assert!(!dir.path().join(format!("{}.part", token.as_str())).exists());
  }

  #[test]
  fn replacing_or_reading_a_missing_file_fails() {
    let dir = tempdir().unwrap();
    let store = LocalMediaStore::new(dir.path());
    let missing = MediaFileToken::new_from_str("mf_local_unope");
    assert!(store.replace_bytes(&missing, b"x").is_err());
    assert!(store.read_text(&missing).is_err());
  }

  #[test]
  fn list_of_a_missing_folder_is_empty() {
    assert!(LocalMediaStore::new("/nonexistent/folder/for/test").list().is_empty());
  }

  #[test]
  fn put_file_copies_and_keeps_extension() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("in.webp");
    fs::write(&source, b"abc").unwrap();
    let store = LocalMediaStore::new(dir.path().join("media"));
    let token = store.put_file(&source).unwrap();

    assert_eq!(store.path_for_token(&token).unwrap().extension().unwrap(), "webp");
    assert!(source.exists());
  }

  #[test]
  fn hosted_and_traversal_tokens_resolve_to_nothing() {
    let dir = tempdir().unwrap();
    let store = LocalMediaStore::new(dir.path());
    store.put_bytes(b"x", "png").unwrap();

    assert!(store.path_for_token(&MediaFileToken::new_from_str("m_abc123")).is_none());
    assert!(store.path_for_token(&MediaFileToken::new_from_str("mf_local_../../secret")).is_none());
    assert!(store.path_for_token(&MediaFileToken::new_from_str("mf_local_")).is_none());
  }

  #[test]
  fn data_uri_has_mime_and_base64() {
    let dir = tempdir().unwrap();
    let store = LocalMediaStore::new(dir.path());
    let token = store.put_bytes(b"hi", "jpg").unwrap();
    assert_eq!(store.data_uri_for_token(&token).unwrap(), "data:image/jpeg;base64,aGk=");
  }

  #[test]
  fn delete_removes_file_once() {
    let dir = tempdir().unwrap();
    let store = LocalMediaStore::new(dir.path());
    let token = store.put_bytes(b"x", "png").unwrap();
    assert!(store.delete(&token).unwrap());
    assert!(!store.delete(&token).unwrap());
    assert!(store.path_for_token(&token).is_none());
  }

  #[test]
  fn asset_url_percent_encodes_the_path() {
    let url = LocalMediaStore::asset_url_for_path(Path::new("/Users/a b/media/x.png"));
    if cfg!(windows) {
      assert!(url.starts_with("http://asset.localhost/"));
    } else {
      assert_eq!(url, "asset://localhost/%2FUsers%2Fa%20b%2Fmedia%2Fx.png");
    }
  }
}
