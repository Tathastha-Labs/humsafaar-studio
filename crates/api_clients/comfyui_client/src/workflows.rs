use serde_json::{json, Value};

/// Parameters for a Stable Diffusion 1.5 style text-to-image run.
#[derive(Clone, Debug)]
pub struct TextToImageParams {
  /// File name inside ComfyUI's `models/checkpoints`.
  pub checkpoint: String,
  pub prompt: String,
  pub negative_prompt: String,
  pub width: u32,
  pub height: u32,
  pub steps: u32,
  pub cfg: f32,
  pub seed: u64,
  pub batch_size: u32,
}

impl TextToImageParams {
  pub fn new(checkpoint: &str, prompt: &str, seed: u64) -> Self {
    Self {
      checkpoint: checkpoint.to_string(),
      prompt: prompt.to_string(),
      negative_prompt: String::new(),
      width: 512,
      height: 512,
      steps: 20,
      cfg: 7.0,
      seed,
      batch_size: 1,
    }
  }
}

/// The workflow in ComfyUI's API format (what `POST /prompt` expects).
/// Node 9 is the save node, whose outputs are read back from the history.
pub fn text_to_image(p: &TextToImageParams) -> Value {
  json!({
    "3": {
      "class_type": "KSampler",
      "inputs": {
        "seed": p.seed,
        "steps": p.steps,
        "cfg": p.cfg,
        "sampler_name": "euler",
        "scheduler": "normal",
        "denoise": 1.0,
        "model": ["4", 0],
        "positive": ["6", 0],
        "negative": ["7", 0],
        "latent_image": ["5", 0]
      }
    },
    "4": { "class_type": "CheckpointLoaderSimple", "inputs": { "ckpt_name": p.checkpoint } },
    "5": {
      "class_type": "EmptyLatentImage",
      "inputs": { "width": p.width, "height": p.height, "batch_size": p.batch_size }
    },
    "6": { "class_type": "CLIPTextEncode", "inputs": { "text": p.prompt, "clip": ["4", 1] } },
    "7": { "class_type": "CLIPTextEncode", "inputs": { "text": p.negative_prompt, "clip": ["4", 1] } },
    "8": { "class_type": "VAEDecode", "inputs": { "samples": ["3", 0], "vae": ["4", 2] } },
    "9": { "class_type": "SaveImage", "inputs": { "filename_prefix": "humsafaar", "images": ["8", 0] } }
  })
}

/// Parameters for Wan 2.2 (5B) video: from text alone, or from a start picture.
#[derive(Clone, Debug)]
pub struct WanVideoParams {
  /// File names inside ComfyUI's `diffusion_models`, `text_encoders` and `vae` folders.
  pub diffusion_model: String,
  pub text_encoder: String,
  pub vae: String,
  pub prompt: String,
  pub negative_prompt: String,
  /// Multiples of 32.
  pub width: u32,
  pub height: u32,
  /// Frames. The model wants a multiple of 4, plus 1.
  pub frames: u32,
  pub fps: f32,
  pub steps: u32,
  pub cfg: f32,
  pub shift: f32,
  pub seed: u64,
  /// The name an image was uploaded under (see `ComfyUiClient::upload_image`). Without it
  /// the video is made from the prompt alone.
  pub start_image: Option<String>,
}

impl WanVideoParams {
  pub fn new(prompt: &str, seed: u64) -> Self {
    Self {
      diffusion_model: "wan2.2_ti2v_5B_fp16.safetensors".to_string(),
      text_encoder: "umt5_xxl_fp8_e4m3fn_scaled.safetensors".to_string(),
      vae: "wan2.2_vae.safetensors".to_string(),
      prompt: prompt.to_string(),
      negative_prompt: String::new(),
      width: 832,
      height: 480,
      frames: 33,
      fps: 24.0,
      steps: 20,
      cfg: 5.0,
      shift: 8.0,
      seed,
      start_image: None,
    }
  }
}

/// The Wan 2.2 5B workflow in ComfyUI's API format. Node 58 saves the MP4.
pub fn wan_video(p: &WanVideoParams) -> Value {
  let mut latent_inputs = json!({
    "vae": ["39", 0],
    "width": p.width,
    "height": p.height,
    "length": p.frames,
    "batch_size": 1
  });

  let mut workflow = json!({
    "37": { "class_type": "UNETLoader", "inputs": { "unet_name": p.diffusion_model, "weight_dtype": "default" } },
    "38": { "class_type": "CLIPLoader", "inputs": { "clip_name": p.text_encoder, "type": "wan", "device": "default" } },
    "39": { "class_type": "VAELoader", "inputs": { "vae_name": p.vae } },
    "48": { "class_type": "ModelSamplingSD3", "inputs": { "model": ["37", 0], "shift": p.shift } },
    "6": { "class_type": "CLIPTextEncode", "inputs": { "text": p.prompt, "clip": ["38", 0] } },
    "7": { "class_type": "CLIPTextEncode", "inputs": { "text": p.negative_prompt, "clip": ["38", 0] } },
    "3": {
      "class_type": "KSampler",
      "inputs": {
        "model": ["48", 0],
        "seed": p.seed,
        "steps": p.steps,
        "cfg": p.cfg,
        "sampler_name": "uni_pc",
        "scheduler": "simple",
        "positive": ["6", 0],
        "negative": ["7", 0],
        "latent_image": ["55", 0],
        "denoise": 1.0
      }
    },
    "8": { "class_type": "VAEDecode", "inputs": { "samples": ["3", 0], "vae": ["39", 0] } },
    "57": { "class_type": "CreateVideo", "inputs": { "images": ["8", 0], "fps": p.fps } },
    "58": {
      "class_type": "SaveVideo",
      "inputs": { "video": ["57", 0], "filename_prefix": "video/humsafaar", "format": "mp4", "codec": "h264" }
    }
  });

  if let Some(image) = &p.start_image {
    latent_inputs["start_image"] = json!(["56", 0]);
    workflow["56"] = json!({ "class_type": "LoadImage", "inputs": { "image": image } });
  }
  workflow["55"] = json!({ "class_type": "Wan22ImageToVideoLatent", "inputs": latent_inputs });
  workflow
}

/// Parameters for ACE-Step 1.5 "turbo" music.
#[derive(Clone, Debug)]
pub struct AceStepParams {
  /// File name inside ComfyUI's `checkpoints` folder.
  pub checkpoint: String,
  /// Style, genre and mood, eg. "warm neo-soul, live drums, mellow".
  pub tags: String,
  /// Sung words with section markers such as "[Verse 1]", or "[instrumental]" for no vocals.
  pub lyrics: String,
  pub seconds: f32,
  pub bpm: u32,
  /// "2", "3", "4" or "6".
  pub time_signature: String,
  pub language: String,
  /// Like "C major" or "E minor".
  pub key_scale: String,
  pub steps: u32,
  pub seed: u64,
}

impl AceStepParams {
  pub fn new(tags: &str, lyrics: &str, seed: u64) -> Self {
    Self {
      checkpoint: "ace_step_1.5_turbo_aio.safetensors".to_string(),
      tags: tags.to_string(),
      lyrics: lyrics.to_string(),
      seconds: 30.0,
      bpm: 120,
      time_signature: "4".to_string(),
      language: "en".to_string(),
      key_scale: "C major".to_string(),
      steps: 8,
      seed,
    }
  }
}

/// The ACE-Step 1.5 turbo workflow in ComfyUI's API format. Node 106 saves the MP3.
pub fn ace_step_music(p: &AceStepParams) -> Value {
  json!({
    "97": { "class_type": "CheckpointLoaderSimple", "inputs": { "ckpt_name": p.checkpoint } },
    "78": { "class_type": "ModelSamplingAuraFlow", "inputs": { "model": ["97", 0], "shift": 3 } },
    "94": {
      "class_type": "TextEncodeAceStepAudio1.5",
      "inputs": {
        "clip": ["97", 1],
        "tags": p.tags,
        "lyrics": p.lyrics,
        "seed": p.seed,
        "bpm": p.bpm,
        "duration": p.seconds,
        "timesignature": p.time_signature,
        "language": p.language,
        "keyscale": p.key_scale,
        "generate_audio_codes": true,
        "cfg_scale": 2.0,
        "temperature": 0.85,
        "top_p": 0.9,
        "top_k": 0,
        "min_p": 0.0
      }
    },
    "47": { "class_type": "ConditioningZeroOut", "inputs": { "conditioning": ["94", 0] } },
    "98": { "class_type": "EmptyAceStep1.5LatentAudio", "inputs": { "seconds": p.seconds, "batch_size": 1 } },
    "3": {
      "class_type": "KSampler",
      "inputs": {
        "model": ["78", 0],
        "seed": p.seed,
        "steps": p.steps,
        "cfg": 1.0,
        "sampler_name": "euler",
        "scheduler": "simple",
        "positive": ["94", 0],
        "negative": ["47", 0],
        "latent_image": ["98", 0],
        "denoise": 1.0
      }
    },
    "18": { "class_type": "VAEDecodeAudio", "inputs": { "samples": ["3", 0], "vae": ["97", 2] } },
    "106": { "class_type": "SaveAudioMP3", "inputs": { "audio": ["18", 0], "filename_prefix": "audio/humsafaar", "quality": "V0" } }
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn fills_in_every_slot() {
    let mut p = TextToImageParams::new("sd15.safetensors", "a red fox", 42);
    p.negative_prompt = "blurry".into();
    p.width = 768;
    p.batch_size = 2;
    let w = text_to_image(&p);

    assert_eq!(w["4"]["inputs"]["ckpt_name"], "sd15.safetensors");
    assert_eq!(w["6"]["inputs"]["text"], "a red fox");
    assert_eq!(w["7"]["inputs"]["text"], "blurry");
    assert_eq!(w["3"]["inputs"]["seed"], 42);
    assert_eq!(w["5"]["inputs"]["width"], 768);
    assert_eq!(w["5"]["inputs"]["batch_size"], 2);
    assert_eq!(w["9"]["class_type"], "SaveImage");
  }

  mod wan {
    use super::*;

    #[test]
    fn text_only_video_has_no_start_image() {
      let w = wan_video(&WanVideoParams::new("a fox runs", 3));
      assert_eq!(w["6"]["inputs"]["text"], "a fox runs");
      assert_eq!(w["55"]["class_type"], "Wan22ImageToVideoLatent");
      assert!(w["55"]["inputs"].get("start_image").is_none());
      assert!(w.get("56").is_none());
      assert_eq!(w["58"]["class_type"], "SaveVideo");
    }

    #[test]
    fn a_start_image_is_loaded_and_wired_into_the_latent() {
      let mut p = WanVideoParams::new("the sea moves", 3);
      p.start_image = Some("lighthouse.png".into());
      let w = wan_video(&p);
      assert_eq!(w["56"]["class_type"], "LoadImage");
      assert_eq!(w["56"]["inputs"]["image"], "lighthouse.png");
      assert_eq!(w["55"]["inputs"]["start_image"], json!(["56", 0]));
    }

    #[test]
    fn size_length_and_sampling_settings_are_filled_in() {
      let mut p = WanVideoParams::new("x", 9);
      p.width = 640;
      p.height = 352;
      p.frames = 25;
      p.steps = 12;
      let w = wan_video(&p);
      assert_eq!(w["55"]["inputs"]["width"], 640);
      assert_eq!(w["55"]["inputs"]["height"], 352);
      assert_eq!(w["55"]["inputs"]["length"], 25);
      assert_eq!(w["3"]["inputs"]["steps"], 12);
      assert_eq!(w["3"]["inputs"]["seed"], 9);
      assert_eq!(w["48"]["inputs"]["shift"], 8.0);
    }

    #[test]
    fn the_three_model_files_are_loaded_by_their_own_nodes() {
      let w = wan_video(&WanVideoParams::new("x", 1));
      assert_eq!(w["37"]["inputs"]["unet_name"], "wan2.2_ti2v_5B_fp16.safetensors");
      assert_eq!(w["38"]["inputs"]["type"], "wan");
      assert_eq!(w["39"]["inputs"]["vae_name"], "wan2.2_vae.safetensors");
    }
  }

  mod ace_step {
    use super::*;

    #[test]
    fn style_lyrics_and_length_are_filled_in() {
      let mut p = AceStepParams::new("warm neo-soul, mellow", "[Verse 1]\nla la", 5);
      p.seconds = 20.0;
      p.bpm = 90;
      let w = ace_step_music(&p);
      assert_eq!(w["94"]["inputs"]["tags"], "warm neo-soul, mellow");
      assert_eq!(w["94"]["inputs"]["lyrics"], "[Verse 1]\nla la");
      assert_eq!(w["94"]["inputs"]["duration"], 20.0);
      assert_eq!(w["98"]["inputs"]["seconds"], 20.0);
      assert_eq!(w["94"]["inputs"]["bpm"], 90);
      assert_eq!(w["3"]["inputs"]["seed"], 5);
      assert_eq!(w["3"]["inputs"]["steps"], 8);
    }

    #[test]
    fn the_checkpoint_feeds_the_sampler_encoder_and_decoder() {
      let w = ace_step_music(&AceStepParams::new("x", "y", 1));
      assert_eq!(w["97"]["inputs"]["ckpt_name"], "ace_step_1.5_turbo_aio.safetensors");
      assert_eq!(w["94"]["inputs"]["clip"], json!(["97", 1]));
      assert_eq!(w["18"]["inputs"]["vae"], json!(["97", 2]));
      assert_eq!(w["106"]["class_type"], "SaveAudioMP3");
    }

    #[test]
    fn lyrics_are_data_not_structure() {
      let w = ace_step_music(&AceStepParams::new("\"}, \"x\": {", "\n[Verse]\n\"quoted\"", 1));
      assert_eq!(w["94"]["inputs"]["tags"], "\"}, \"x\": {");
    }
  }

  #[test]
  fn prompt_text_is_data_not_structure() {
    let p = TextToImageParams::new("m", "\"}, \"x\": {", 1);
    assert_eq!(text_to_image(&p)["6"]["inputs"]["text"], "\"}, \"x\": {");
  }
}
