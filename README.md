<h1 align="center">Humsafaar Studio</h1>
<p align="center"><strong>A local studio for writing scripts and making images, video, audio and 3D with your own API keys.</strong></p>

Humsafaar Studio is a desktop app (Tauri 2 and React). It runs on your computer: no account, no hosted
service. You add your own provider keys, and everything you make stays on your disk.

## What it does

- **Script:** break a story into shots, edit them by hand or with AI, speak them, and make a picture and
  a video for each one.
- **Create:** images, video, music, and 3D objects, on your own Fal, Gemini, Grok, GMI Cloud, Replicate
  or local ComfyUI.
- **Edit:** image editor, background removal, frame extractor, moodboard, and a 3D stage.

## Run it

Needs Node.js 20+, Rust, `cargo-tauri` 2 and the Tauri platform prerequisites.

```bash
./script/humsafaar/unix_dev.sh
```

The dev app keeps its keys, media and history in `~/Humsafaar-Studio`. Set `HUMSAFAAR_DATA_DIR` to use a
different folder. Add provider keys in **Settings, then API Keys**.

## Credits and licence

Built on [Humsafaar Studio](https://github.com/humsafaar/studio) by the Humsafaar Studio Team, under the MIT or
Apache-2.0 licence (`LICENSE-MIT`, `LICENSE-APACHE`). The Humsafaar Studio name and logos are not covered by that
licence and are not used here. Third-party material is listed in `NOTICE`.
