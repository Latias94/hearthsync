use std::ffi::OsString;
use std::sync::{Mutex, MutexGuard};

pub(super) fn curseforge_api_key_guard(value: &str) -> CurseForgeApiKeyGuard {
    curseforge_api_key_env_guard(Some(value), None)
}

pub(super) fn standard_curseforge_api_key_guard(value: &str) -> CurseForgeApiKeyGuard {
    curseforge_api_key_env_guard(None, Some(value))
}

fn curseforge_api_key_env_guard(
    hearthsync_value: Option<&str>,
    standard_value: Option<&str>,
) -> CurseForgeApiKeyGuard {
    static CURSEFORGE_API_KEY_ENV_MUTEX: Mutex<()> = Mutex::new(());
    let lock = CURSEFORGE_API_KEY_ENV_MUTEX
        .lock()
        .expect("curseforge api key env lock");
    let hearthsync_key = "HEARTHSYNC_CURSEFORGE_API_KEY";
    let standard_key = "CURSEFORGE_API_KEY";
    let previous_hearthsync = std::env::var_os(hearthsync_key);
    let previous_standard = std::env::var_os(standard_key);
    set_optional_env_var(hearthsync_key, hearthsync_value);
    set_optional_env_var(standard_key, standard_value);

    CurseForgeApiKeyGuard {
        hearthsync_key,
        previous_hearthsync,
        standard_key,
        previous_standard,
        _lock: lock,
    }
}

pub(super) struct CurseForgeApiKeyGuard {
    hearthsync_key: &'static str,
    previous_hearthsync: Option<OsString>,
    standard_key: &'static str,
    previous_standard: Option<OsString>,
    _lock: MutexGuard<'static, ()>,
}

impl Drop for CurseForgeApiKeyGuard {
    fn drop(&mut self) {
        restore_env_var(self.hearthsync_key, &self.previous_hearthsync);
        restore_env_var(self.standard_key, &self.previous_standard);
    }
}

fn set_optional_env_var(key: &str, value: Option<&str>) {
    match value {
        Some(value) => unsafe {
            std::env::set_var(key, value);
        },
        None => unsafe {
            std::env::remove_var(key);
        },
    }
}

fn restore_env_var(key: &str, value: &Option<OsString>) {
    match value {
        Some(value) => unsafe {
            std::env::set_var(key, value);
        },
        None => unsafe {
            std::env::remove_var(key);
        },
    }
}
