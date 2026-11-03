use crate::core::state::humsafaar_usage_tracker::humsafaar_usage_tracker_data::HumsafaarUsageTrackerData;
use std::sync::{Arc, LockResult, RwLock};
use log::{error, warn};
use crate::core::humsafaar_error::HumsafaarError;
use crate::core::state::humsafaar_usage_tracker::humsafaar_usage_type::{HumsafaarUsagePage, HumsafaarUsageType};

/// Track generation volume so we know which areas to focus efforts on.
/// But also note: https://en.wikipedia.org/wiki/Survivorship_bias
#[derive(Clone)]
pub struct HumsafaarUsageTracker {
  data: Arc<RwLock<HumsafaarUsageTrackerData>>,
}


impl HumsafaarUsageTracker {
  pub fn new() -> Self {
    Self {
      data: Arc::new(RwLock::new(HumsafaarUsageTrackerData::new())),
    }
  }
  
  pub fn record_image_generation(&self, num_images: u16, usage_type: HumsafaarUsageType, page: HumsafaarUsagePage) -> Result<(), HumsafaarError> {
    let mut data = self.get()?;
    data.total_generation_count += num_images;
    data.image_generation_count += num_images;
    match usage_type {
      HumsafaarUsageType::TextToResult => {
        data.text_to_image_count += num_images;
      }
      HumsafaarUsageType::ImageToResult => {
        data.image_to_image_count += num_images;
      }
      HumsafaarUsageType::Other => {}
    }
    match page {
      HumsafaarUsagePage::ImagePage => {
        data.image_page_prompt_count += num_images;
      }
      HumsafaarUsagePage::EditPage => {
        data.edit_page_prompt_count += num_images;
      }
      HumsafaarUsagePage::StagePage => {
        data.stage_page_prompt_count += num_images;
      }
      HumsafaarUsagePage::OtherPage => {
        data.other_page_prompt_count += num_images;
      }
      HumsafaarUsagePage::VideoPage => {} // NB: Does not generate images.
      HumsafaarUsagePage::ObjectPage => {} // NB: Does not generate images.
    }
    self.set(data)?;
    Ok(())
  }
  
  pub fn record_video_generation(&self, num_videos: u16, usage_type: HumsafaarUsageType, page: HumsafaarUsagePage) -> Result<(), HumsafaarError> {
    let mut data = self.get()?;
    data.total_generation_count += num_videos;
    data.video_generation_count += num_videos;
    match usage_type {
      HumsafaarUsageType::TextToResult => {
        data.text_to_video_count += num_videos;
      }
      HumsafaarUsageType::ImageToResult => {
        data.image_to_video_count += num_videos;
      }
      HumsafaarUsageType::Other => {}
    }
    match page {
      HumsafaarUsagePage::VideoPage => {
        data.video_page_prompt_count += num_videos;
      }
      HumsafaarUsagePage::OtherPage => {
        data.other_page_prompt_count += num_videos;
      }
      HumsafaarUsagePage::ImagePage => {} // NB: Does not generate videos.
      HumsafaarUsagePage::EditPage => {} // NB: Does not generate videos.
      HumsafaarUsagePage::StagePage => {} // NB: Does not generate videos.
      HumsafaarUsagePage::ObjectPage => {} // NB: Does not generate videos.
    }
    self.set(data)?;
    Ok(())
  }
  
  pub fn record_object_generation(&self, num_objects: u16, usage_type: HumsafaarUsageType, page: HumsafaarUsagePage) -> Result<(), HumsafaarError> {
    let mut data = self.get()?;
    data.total_generation_count += num_objects;
    data.object_generation_count += num_objects;
    match usage_type {
      HumsafaarUsageType::TextToResult => {
        data.text_to_object_count += num_objects;
      }
      HumsafaarUsageType::ImageToResult => {
        data.image_to_object_count += num_objects;
      }
      HumsafaarUsageType::Other => {}
    }
    match page {
      HumsafaarUsagePage::ObjectPage => {
        data.object_page_prompt_count += num_objects;
      }
      HumsafaarUsagePage::OtherPage => {
        data.other_page_prompt_count += num_objects;
      }
      HumsafaarUsagePage::ImagePage => {} // NB: Does not generate objects.
      HumsafaarUsagePage::VideoPage => {} // NB: Does not generate objects.
      HumsafaarUsagePage::EditPage => {} // NB: Does not generate objects.
      HumsafaarUsagePage::StagePage => {} // NB: Does not generate objects.
    }
    self.set(data)?;
    Ok(())
  }
  
  pub fn get(&self) -> Result<HumsafaarUsageTrackerData, HumsafaarError> {
    match self.data.read() {
      Ok(data) => Ok(data.clone()),
      Err(err) => {
        error!("Lock read error: {:?}", err);
        Err(HumsafaarError::RwLockReadError)
      }
    }
  }
  
  fn set(&self, data: HumsafaarUsageTrackerData) -> Result<(), HumsafaarError> {
    match self.data.write() {
      Ok(mut lock) => {
        *lock = data;
        Ok(())
      },
      Err(err) => {
        error!("Lock write error: {:?}", err);
        Err(HumsafaarError::RwLockWriteError)
      }
    }
  }
}
