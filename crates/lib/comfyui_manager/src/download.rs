use crate::error::{io_error, ManagerError, ManagerResult};
use crate::platform::ArchiveKind;
use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use tokio::io::AsyncWriteExt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Progress {
  pub downloaded: u64,
  pub total: Option<u64>,
}

/// Download `url` to `destination`, resuming a partial `.part` file if there is one.
/// When `sha256` is given, the file only appears at `destination` once it matches.
pub async fn download_verified(
  url: &str,
  destination: &Path,
  sha256: Option<&str>,
  on_progress: &mut (dyn FnMut(Progress) + Send),
) -> ManagerResult<()> {
  if let Some(parent) = destination.parent() {
    tokio::fs::create_dir_all(parent).await.map_err(|e| io_error(parent, e))?;
  }

  let part = part_path(destination);
  let already = tokio::fs::metadata(&part).await.map(|m| m.len()).unwrap_or(0);

  let client = reqwest::Client::new();
  let mut request = client.get(url);
  if already > 0 {
    request = request.header("Range", format!("bytes={}-", already));
  }
  let response = request.send().await?;
  let status = response.status();

  let (mut downloaded, mut file) = if status == reqwest::StatusCode::PARTIAL_CONTENT {
    let file = tokio::fs::OpenOptions::new().append(true).open(&part).await
      .map_err(|e| io_error(&part, e))?;
    (already, file)
  } else if status.is_success() {
    // The server sent the whole file (or we had nothing yet): start over.
    (0, tokio::fs::File::create(&part).await.map_err(|e| io_error(&part, e))?)
  } else if status == reqwest::StatusCode::RANGE_NOT_SATISFIABLE && already > 0 {
    // The partial file is not usable; discard it and try once from scratch.
    let _ = tokio::fs::remove_file(&part).await;
    return Box::pin(download_verified(url, destination, sha256, on_progress)).await;
  } else {
    return Err(ManagerError::DownloadStatus { url: url.to_string(), status: status.as_u16() });
  };

  let total = response.content_length().map(|remaining| remaining + downloaded);
  on_progress(Progress { downloaded, total });

  let mut stream = response.bytes_stream();
  while let Some(chunk) = stream.next().await {
    let chunk = chunk?;
    file.write_all(&chunk).await.map_err(|e| io_error(&part, e))?;
    downloaded += chunk.len() as u64;
    on_progress(Progress { downloaded, total });
  }
  file.flush().await.map_err(|e| io_error(&part, e))?;
  drop(file);

  if let Some(expected) = sha256 {
    let actual = sha256_of_file(&part).await?;
    if !actual.eq_ignore_ascii_case(expected) {
      let _ = tokio::fs::remove_file(&part).await;
      return Err(ManagerError::ChecksumMismatch {
        file: destination.display().to_string(),
        expected: expected.to_string(),
        actual,
      });
    }
  }

  tokio::fs::rename(&part, destination).await.map_err(|e| io_error(destination, e))?;
  Ok(())
}

pub async fn sha256_of_file(path: &Path) -> ManagerResult<String> {
  let path = path.to_path_buf();
  tokio::task::spawn_blocking(move || -> ManagerResult<String> {
    let mut file = File::open(&path).map_err(|e| io_error(&path, e))?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 1 << 20];
    loop {
      let read = file.read(&mut buffer).map_err(|e| io_error(&path, e))?;
      if read == 0 {
        break;
      }
      hasher.update(&buffer[..read]);
    }
    Ok(hasher.finalize().iter().map(|b| format!("{:02x}", b)).collect())
  })
  .await
  .map_err(|e| ManagerError::Other(e.to_string()))?
}

/// Unpack an archive into `destination`, dropping the first `strip_components`
/// path segments (like `tar --strip-components`). Entries that would land
/// outside `destination`, and links, are refused or skipped.
pub async fn extract_archive(
  kind: ArchiveKind,
  archive: &Path,
  destination: &Path,
  strip_components: usize,
) -> ManagerResult<()> {
  let archive = archive.to_path_buf();
  let destination = destination.to_path_buf();
  tokio::task::spawn_blocking(move || match kind {
    ArchiveKind::TarGz => extract_tar_gz(&archive, &destination, strip_components),
    ArchiveKind::Zip => extract_zip(&archive, &destination, strip_components),
  })
  .await
  .map_err(|e| ManagerError::Other(e.to_string()))?
}

fn extract_tar_gz(archive: &Path, destination: &Path, strip: usize) -> ManagerResult<()> {
  let file = File::open(archive).map_err(|e| io_error(archive, e))?;
  let mut tar = tar::Archive::new(flate2::read::GzDecoder::new(file));
  std::fs::create_dir_all(destination).map_err(|e| io_error(destination, e))?;

  for entry in tar.entries().map_err(archive_error)? {
    let mut entry = entry.map_err(archive_error)?;
    let kind = entry.header().entry_type();
    if !(kind.is_file() || kind.is_dir()) {
      continue;
    }
    let path = entry.path().map_err(archive_error)?.into_owned();
    let Some(relative) = safe_relative_path(&path, strip)? else { continue };
    let target = destination.join(&relative);

    if kind.is_dir() {
      std::fs::create_dir_all(&target).map_err(|e| io_error(&target, e))?;
      continue;
    }
    if let Some(parent) = target.parent() {
      std::fs::create_dir_all(parent).map_err(|e| io_error(parent, e))?;
    }
    entry.unpack(&target).map_err(archive_error)?;
  }
  Ok(())
}

fn extract_zip(archive: &Path, destination: &Path, strip: usize) -> ManagerResult<()> {
  let file = File::open(archive).map_err(|e| io_error(archive, e))?;
  let mut zip = zip::ZipArchive::new(file).map_err(archive_error)?;
  std::fs::create_dir_all(destination).map_err(|e| io_error(destination, e))?;

  for index in 0..zip.len() {
    let mut entry = zip.by_index(index).map_err(archive_error)?;
    let name = entry.name().to_string();
    let Some(enclosed) = entry.enclosed_name() else {
      return Err(ManagerError::UnsafeArchivePath(name));
    };
    let Some(relative) = safe_relative_path(&enclosed, strip)? else { continue };
    let target = destination.join(&relative);

    if entry.is_dir() {
      std::fs::create_dir_all(&target).map_err(|e| io_error(&target, e))?;
      continue;
    }
    if let Some(parent) = target.parent() {
      std::fs::create_dir_all(parent).map_err(|e| io_error(parent, e))?;
    }
    let mut out = File::create(&target).map_err(|e| io_error(&target, e))?;
    std::io::copy(&mut entry, &mut out).map_err(|e| io_error(&target, e))?;
  }
  Ok(())
}

/// `None` when nothing is left after stripping; an error for any path that
/// could leave the destination.
fn safe_relative_path(path: &Path, strip: usize) -> ManagerResult<Option<PathBuf>> {
  let mut kept = PathBuf::new();
  for (index, component) in path.components().enumerate() {
    match component {
      Component::Normal(part) => {
        if index >= strip {
          kept.push(part);
        }
      }
      Component::CurDir => {}
      _ => return Err(ManagerError::UnsafeArchivePath(path.display().to_string())),
    }
  }
  Ok(if kept.as_os_str().is_empty() { None } else { Some(kept) })
}

fn part_path(destination: &Path) -> PathBuf {
  let mut name = destination.file_name().unwrap_or_default().to_os_string();
  name.push(".part");
  destination.with_file_name(name)
}

fn archive_error(error: impl std::fmt::Display) -> ManagerError {
  ManagerError::Archive(error.to_string())
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::io::Write;
  use std::sync::{Arc, Mutex};
  use tempfile::tempdir;
  use tokio::io::AsyncReadExt;
  use tokio::net::TcpListener;

  const BODY: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";

  mod download {
    use super::*;

    #[tokio::test]
    async fn downloads_and_verifies_the_checksum() {
      let url = serve(BODY.to_vec()).await;
      let dir = tempdir().unwrap();
      let destination = dir.path().join("sub").join("file.bin");

      let updates = Arc::new(Mutex::new(Vec::new()));
      let sink = updates.clone();
      download_verified(&url, &destination, Some(&sha256_hex(BODY)), &mut move |p| {
        sink.lock().unwrap().push(p);
      }).await.unwrap();

      assert_eq!(std::fs::read(&destination).unwrap(), BODY);
      assert!(!part_path(&destination).exists());
      let last = *updates.lock().unwrap().last().unwrap();
      assert_eq!(last, Progress { downloaded: BODY.len() as u64, total: Some(BODY.len() as u64) });
    }

    #[tokio::test]
    async fn a_wrong_checksum_leaves_no_file_behind() {
      let url = serve(BODY.to_vec()).await;
      let dir = tempdir().unwrap();
      let destination = dir.path().join("file.bin");

      let err = download_verified(&url, &destination, Some(&"0".repeat(64)), &mut |_| {})
        .await.unwrap_err();

      assert!(matches!(err, ManagerError::ChecksumMismatch { .. }), "{err:?}");
      assert!(!destination.exists());
      assert!(!part_path(&destination).exists());
    }

    #[tokio::test]
    async fn resumes_a_partial_download() {
      let url = serve(BODY.to_vec()).await;
      let dir = tempdir().unwrap();
      let destination = dir.path().join("file.bin");
      std::fs::write(part_path(&destination), &BODY[..10]).unwrap();

      download_verified(&url, &destination, Some(&sha256_hex(BODY)), &mut |_| {})
        .await.unwrap();

      assert_eq!(std::fs::read(&destination).unwrap(), BODY);
    }

    #[tokio::test]
    async fn an_error_status_is_reported() {
      let url = serve_status(404).await;
      let dir = tempdir().unwrap();
      let err = download_verified(&url, &dir.path().join("f"), None, &mut |_| {})
        .await.unwrap_err();
      assert!(matches!(err, ManagerError::DownloadStatus { status: 404, .. }), "{err:?}");
    }
  }

  mod extract {
    use super::*;

    #[tokio::test]
    async fn tar_gz_strips_the_top_folder() {
      let dir = tempdir().unwrap();
      let archive = dir.path().join("a.tar.gz");
      write_tar_gz(&archive, &[("proj-1.0/main.py", b"print(1)"), ("proj-1.0/lib/util.py", b"x")]);

      let out = dir.path().join("out");
      extract_archive(ArchiveKind::TarGz, &archive, &out, 1).await.unwrap();

      assert_eq!(std::fs::read(out.join("main.py")).unwrap(), b"print(1)");
      assert_eq!(std::fs::read(out.join("lib/util.py")).unwrap(), b"x");
    }

    #[tokio::test]
    async fn tar_gz_with_a_parent_path_is_refused() {
      let dir = tempdir().unwrap();
      let archive = dir.path().join("evil.tar.gz");
      write_tar_gz_raw_name(&archive, "../escaped.txt", b"nope");

      let out = dir.path().join("out");
      let err = extract_archive(ArchiveKind::TarGz, &archive, &out, 0).await.unwrap_err();

      assert!(matches!(err, ManagerError::UnsafeArchivePath(_)), "{err:?}");
      assert!(!dir.path().join("escaped.txt").exists());
    }

    #[tokio::test]
    async fn zip_extracts_a_flat_archive() {
      let dir = tempdir().unwrap();
      let archive = dir.path().join("a.zip");
      write_zip(&archive, &[("uv.exe", b"binary"), ("docs/readme.txt", b"hi")]);

      let out = dir.path().join("out");
      extract_archive(ArchiveKind::Zip, &archive, &out, 0).await.unwrap();

      assert_eq!(std::fs::read(out.join("uv.exe")).unwrap(), b"binary");
      assert_eq!(std::fs::read(out.join("docs/readme.txt")).unwrap(), b"hi");
    }

    #[test]
    fn safe_relative_path_rules() {
      assert_eq!(safe_relative_path(Path::new("a/b/c"), 1).unwrap(), Some(PathBuf::from("b/c")));
      assert_eq!(safe_relative_path(Path::new("a"), 1).unwrap(), None);
      assert!(safe_relative_path(Path::new("a/../../b"), 0).is_err());
      assert!(safe_relative_path(Path::new("/etc/passwd"), 0).is_err());
    }
  }

  #[tokio::test]
  async fn sha256_of_file_matches_the_known_digest() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("f");
    std::fs::write(&path, b"abc").unwrap();
    assert_eq!(
      sha256_of_file(&path).await.unwrap(),
      "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
    );
  }

  // ── helpers ──

  fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{:02x}", b)).collect()
  }

  fn write_tar_gz(path: &Path, files: &[(&str, &[u8])]) {
    let encoder = flate2::write::GzEncoder::new(File::create(path).unwrap(), flate2::Compression::fast());
    let mut builder = tar::Builder::new(encoder);
    for (name, data) in files {
      let mut header = tar::Header::new_gnu();
      header.set_size(data.len() as u64);
      header.set_mode(0o644);
      header.set_cksum();
      builder.append_data(&mut header, name, *data).unwrap();
    }
    builder.into_inner().unwrap().finish().unwrap();
  }

  /// `tar::Builder` refuses `..`, so write the name bytes directly.
  fn write_tar_gz_raw_name(path: &Path, name: &str, data: &[u8]) {
    let encoder = flate2::write::GzEncoder::new(File::create(path).unwrap(), flate2::Compression::fast());
    let mut builder = tar::Builder::new(encoder);
    let mut header = tar::Header::new_gnu();
    header.set_size(data.len() as u64);
    header.set_mode(0o644);
    header.as_old_mut().name[..name.len()].copy_from_slice(name.as_bytes());
    header.set_cksum();
    builder.append(&header, data).unwrap();
    builder.into_inner().unwrap().finish().unwrap();
  }

  fn write_zip(path: &Path, files: &[(&str, &[u8])]) {
    let mut writer = zip::ZipWriter::new(File::create(path).unwrap());
    let options = zip::write::SimpleFileOptions::default();
    for (name, data) in files {
      writer.start_file(*name, options).unwrap();
      writer.write_all(data).unwrap();
    }
    writer.finish().unwrap();
  }

  async fn serve(body: Vec<u8>) -> String {
    serve_with(move |range_start| match range_start {
      Some(start) if start < body.len() => (206, body[start..].to_vec()),
      Some(_) => (416, Vec::new()),
      None => (200, body.clone()),
    }).await
  }

  async fn serve_status(status: u16) -> String {
    serve_with(move |_| (status, Vec::new())).await
  }

  /// A one-route HTTP server. The closure maps an optional `Range` start to (status, body).
  async fn serve_with<F>(respond: F) -> String
  where F: Fn(Option<usize>) -> (u16, Vec<u8>) + Send + Sync + 'static {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/file", listener.local_addr().unwrap());
    let respond = Arc::new(respond);
    tokio::spawn(async move {
      loop {
        let Ok((mut socket, _)) = listener.accept().await else { return };
        let respond = respond.clone();
        tokio::spawn(async move {
          let mut buffer = vec![0u8; 4096];
          let read = socket.read(&mut buffer).await.unwrap_or(0);
          let request = String::from_utf8_lossy(&buffer[..read]).to_string();
          let range_start = request.lines()
            .find_map(|l| l.to_ascii_lowercase().strip_prefix("range: bytes=").map(str::to_string))
            .and_then(|r| r.trim_end_matches('-').parse::<usize>().ok());
          let (status, body) = respond(range_start);
          let head = format!(
            "HTTP/1.1 {} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            status, body.len());
          let _ = socket.write_all(head.as_bytes()).await;
          let _ = socket.write_all(&body).await;
          let _ = socket.shutdown().await;
        });
      }
    });
    url
  }
}
