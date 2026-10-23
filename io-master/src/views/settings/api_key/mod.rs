//! The element "API key" of the view Settings, tab AI: the key of one
//! connection (page: `ui/src/views/settings/ApiKey.svelte`).
//!
//! ```text
//! action   command          parameters    data back
//! read     api_key_get      id            ApiKeyView: whether a key is stored, whether a header uses it
//! store    api_key_store    id, api_key   ApiKeyView
//! delete   api_key_delete   id            ApiKeyView
//! ```
//!
//! **The key is never sent back to the page**: it is kept only in the
//! encrypted database of the evaluator (`db/ai/connection/`), bound to the
//! address of the endpoint it was entered for. The page sends it once,
//! from a password field, and clears it.

pub mod commands;
pub mod view;
