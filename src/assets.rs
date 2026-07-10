pub const GRUNT_ICON: &[u8] = include_bytes!("../assets/icons/logo.ico");
pub const VSAPI_VERSIONS: &str = "https://api.vintagestory.at/stable.json";
pub const VSMODDB: &str = "https://mods.vintagestory.at/api/";
pub const VSAUTH: &str = "https://auth3.vintagestory.at/v2/gamelogin";
#[cfg(target_os = "windows")]
pub const VSWINREGKEY: &str = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\{70364653-036D-49B3-8B80-AF39665F29C1}_is1";
