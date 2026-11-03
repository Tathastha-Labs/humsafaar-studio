pub mod handle_router;
mod handle_api_providers;
mod handle_grok;
mod handle_gemini;
pub use handle_gemini::LOCAL_FILE_SCHEME;
pub use handle_grok::RESULT_URL_SEPARATOR;
pub(super) mod utils;
