use once_cell::sync::OnceCell;

pub mod methods;
pub mod models;

use crate::settings::models::Settings;

pub static SETTINGS: OnceCell<Settings> = OnceCell::new();

pub fn settings() -> &'static Settings {
    SETTINGS.get().expect("config init")
}

pub fn init() {
    let settings: Settings = Settings::new().unwrap();
    SETTINGS
        .set(settings)
        .expect("Somehow Darth Sidious has returned!");
}
