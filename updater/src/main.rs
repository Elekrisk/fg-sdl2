use std::{io::Cursor, time::SystemTime, path::{Path, PathBuf}, fs::FileType};

use fs_extra::dir::{CopyOptions, DirOptions};
use reqwest::{blocking::Request, Method, Url};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Config {
    current_release: String,
    last_check: u64,
    filename: String,
}

macro_rules! exit {
    () => {
        println!("Press Enter to exit");
        std::io::stdin().read_line(&mut String::new()).unwrap();
        return;
    };
}

fn main() {
    let mut config: Config =
        serde_json::from_slice(&std::fs::read("./config/autoupdate.json").unwrap()).unwrap();

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let diff = now - config.last_check;

    if diff < 60 {
        println!("Time since last update check is {diff} seconds, which is less than the 1 per min rate limit.");
        println!(
            "Please wait another {} seconds before checking again.",
            60 - diff
        );
        exit!();
    }

    config.last_check = now;
    std::fs::write(
        "./config/autoupdate.json",
        &serde_json::to_vec(&config).unwrap(),
    )
    .unwrap();

    let client = reqwest::blocking::Client::builder()
        .user_agent("elekrisk_fg-sdl2_updater")
        .build()
        .unwrap();

    let request = Request::new(
        Method::GET,
        Url::parse("https://api.github.com/repos/elekrisk/fg-sdl2/git/ref/tags/latest").unwrap(),
    );
    let response = client.execute(request).unwrap();

    let value: serde_json::Value = response.json().unwrap();
    let sha = value.as_object().unwrap()["object"].as_object().unwrap()["sha"]
        .as_str()
        .unwrap();

    if config.current_release == sha {
        println!("No update found.");
        exit!();
    } else {
        println!("Update found!");
    }

    let response = client
        .get("https://api.github.com/repos/elekrisk/fg-sdl2/releases/tags/latest")
        .send()
        .unwrap();
    let value: serde_json::Value = response.json().unwrap();
    let assets = value.as_object().unwrap()["assets"].as_array().unwrap();
    let Some(zip_link) = assets.iter().find_map(|a| {
        if a.as_object().unwrap()["name"].as_str().unwrap() == config.filename {
            Some(
                a.as_object().unwrap()["browser_download_url"]
                    .as_str()
                    .unwrap(),
            )
        } else {
            None
        }
    }) else {
        println!("No release for {} found.", config.filename);
        exit!();
    };

    let response = client.get(zip_link).send().unwrap();
    let zip_file = response.bytes().unwrap();
    let mut zip_file = zip::ZipArchive::new(Cursor::new(zip_file)).unwrap();
    std::fs::create_dir("temp").unwrap();
    zip_file.extract("./temp").unwrap();
    let mut entries = vec![];
    for entry in std::fs::read_dir("temp").unwrap() {
        let entry = entry.unwrap();
        if entry.file_name() == "config" {
            std::fs::remove_dir_all(entry.path()).unwrap();
            continue;
        } else {
            entries.push(entry.path());
        }

        // let file_type = entry.file_type().unwrap();
        // if file_type.is_dir() {
        //     copy_dir(&PathBuf::from("temp").join(entry.file_name()), &PathBuf::from(entry.file_name()));
        // } else if file_type.is_file() {
        //     std::fs::copy(entry.path(), entry.file_name()).unwrap();
        // }
    }

    fs_extra::copy_items(&entries, ".", &CopyOptions::new().overwrite(true)).unwrap();

    std::fs::remove_dir_all("temp").unwrap();

    config.current_release = sha.into();
    std::fs::write(
        "./config/autoupdate.json",
        &serde_json::to_vec(&config).unwrap(),
    )
    .unwrap();
    
    println!("Updated to {sha}");
    exit!();
}

fn copy_dir(src: &Path, dst: &Path) {
    for entry in std::fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let file_type = entry.file_type().unwrap();
        let src = PathBuf::from(src).join(entry.file_name());
        let dst = PathBuf::from(dst).join(entry.file_name());
        if file_type.is_dir() {
            std::fs::create_dir(&dst).unwrap();
            copy_dir(&src, &dst);
        } else if file_type.is_file() {
            std::fs::copy(&src, &dst).unwrap();
        }
    }
}
