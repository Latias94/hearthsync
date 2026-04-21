use serde::Serialize;

use crate::core::error::AppResult;

pub(super) mod addon;
pub(super) mod addon_lock;
pub(super) mod backup;
pub(super) mod bundle;
pub(super) mod config;
pub(super) mod external_package;
pub(super) mod shared;
pub(super) mod system;
#[cfg(test)]
mod test_support;

pub(super) fn render<T, F>(json: bool, value: &T, text_renderer: F) -> AppResult<()>
where
    T: Serialize,
    F: FnOnce(&T) -> String,
{
    if json {
        println!("{}", serde_json::to_string_pretty(value)?);
    } else {
        println!("{}", text_renderer(value));
    }

    Ok(())
}
