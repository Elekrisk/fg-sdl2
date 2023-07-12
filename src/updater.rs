use std::path::PathBuf;

use serde::{Serialize, Deserialize};


pub struct Updater {
    config: Config
}

impl Updater {
    pub fn new() -> Self {
        Self {
            config: Config::load_or_generate()
        }
    }
}

#[derive(Serialize, Deserialize)]
struct Config {
    current_release: String,
    last_check: u64,
    filename: String,
}

#[cfg(windows)]
const FILENAME: &'static str = "windows_debug_x86_64.zip";
#[cfg(unix)]
const FILENAME: &'static str = "linux_debug_x86_64.zip";

impl Config {
    fn generate() -> Self {
        let filename = FILENAME.into();

        Self {
            current_release: String::new(),
            last_check: 0,
            filename,
        }
    }

    fn load() -> Self {
        serde_json::from_slice(&std::fs::read("./config/autoupdate.json").unwrap()).unwrap()
    }

    fn load_or_generate() -> Self {
        let path = PathBuf::from("./config/autoupdate.json");
        if path.exists() {
            Self::load()
        } else {
            Self::generate()
        }
    }
}


