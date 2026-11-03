import { JobStatus } from "../enums/Job.js";

export interface Job {
  created_at: string;
  job_token: string;
  maybe_result: JobMaybeResult;
  request: JobRequest;
  status: JobStatusResponse;
  updated_at: string;
}

export interface JobMaybeResult {
  entity_token: string;
  entity_type: string;
  // maybe_public_bucket_media_path: string;
  maybe_successfully_completed_at: string;
  media_links: JobMediaLinks;
}

export interface JobMediaLinks {
  cdn_url: string;
  maybe_thumbnail_template?: string | null;
  maybe_video_previews?: JobMediaLinksVideoPreviews | null;
}

export interface JobMediaLinksVideoPreviews {
  still: string;
  animated: string;
  still_thumbnail_template: string;
  animated_thumbnail_template: string;
}
export interface JobRequest {
  inference_category: string;
  maybe_model_title: string;
  maybe_model_token: string;
  maybe_model_type: string;
  maybe_prompt_token: string | null;
  maybe_raw_inference_text: string;
  maybe_style_name: string;
}

export interface JobStatusResponse {
  attempt_count: number;
  maybe_assigned_cluster: string;
  maybe_assigned_worker: string;
  maybe_extra_status_description: string;
  maybe_failure_category: string;
  maybe_failure_category_updated: string | null;
  maybe_failure_message: string | null;
  maybe_first_started_at: string;
  requires_keepalive: boolean;
  progress_percentage: number;
  status: JobStatus;
}

