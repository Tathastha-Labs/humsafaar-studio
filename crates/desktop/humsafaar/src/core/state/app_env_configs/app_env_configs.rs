use crate::core::state::data_dir::app_data_root::AppDataRoot;
use errors::AnyhowResult;

/// Settings read from the environment at startup. There are none now that nothing is hosted, but the
/// type is kept so handlers that take it do not have to change.
#[derive(Clone, Default)]
pub struct AppEnvConfigs {}

impl AppEnvConfigs {
  pub fn load_from_filesystem(_root: &AppDataRoot) -> AnyhowResult<Self> {
    Ok(Self {})
  }
}
