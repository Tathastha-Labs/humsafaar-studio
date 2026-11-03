//! Shared helpers for rejecting builder options that a model does not support.
//!
//! Per the request-mismatch mitigation strategy: `ErrorOut` rejects the
//! request when an unsupported option is set; the other strategies silently
//! drop the option.

use std::fmt::Debug;

use crate::api::image_ref::ImageRef;
use crate::client::request_mismatch_mitigation_strategy::RequestMismatchMitigationStrategy;
use crate::errors::humsafaar_router_error::HumsafaarRouterError;
use crate::errors::client_error::ClientError;

/// Reject (under `ErrorOut`) or silently drop a set option the model doesn't
/// support.
pub(crate) fn reject_unsupported_option<T: Debug>(
  field: &'static str,
  value: Option<&T>,
  strategy: RequestMismatchMitigationStrategy,
) -> Result<(), HumsafaarRouterError> {
  let Some(value) = value else {
    return Ok(());
  };
  match strategy {
    RequestMismatchMitigationStrategy::ErrorOut => {
      Err(HumsafaarRouterError::Client(ClientError::ModelDoesNotSupportOption {
        field,
        value: format!("{:?}", value),
      }))
    }
    RequestMismatchMitigationStrategy::PayMoreUpgrade
    | RequestMismatchMitigationStrategy::PayLessDowngrade => Ok(()),
  }
}

/// Reject (under `ErrorOut`) or silently drop a single side-view image for
/// models/endpoints that take none.
pub(crate) fn reject_unsupported_image_ref(
  field: &'static str,
  image_ref: Option<&ImageRef>,
  strategy: RequestMismatchMitigationStrategy,
) -> Result<(), HumsafaarRouterError> {
  reject_unsupported_option(field, image_ref, strategy)
}
