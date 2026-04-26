mod crate_manager;
mod args;
mod config;
mod downloader;
mod handler;
mod krate;
mod crate_toml_config;
mod unzip;
mod utils;

use std::{
    env::home_dir,
    fs, io,
    path::{Path, PathBuf},
};

use clap::Parser;
use once_cell::sync::Lazy;

use crate::{
    args::Command,
    config::Config,
    handler::{
        add_crate_handler::add_crate_handler, compile_project_handler::compile_project_handler,
        create_project_handler::create_project_handler,
    },
};

const NORMAL_CONFIG: &str = r#"{
    "mod_index": "https://raw.githubusercontent.com/ant-pl/ant-crate/main/crates.json"
}"#;

fn init() -> Result<(), io::Error> {
    let mut antman_path = home_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "cannot found home path"))?;

    antman_path.push(".antman");

    fs::create_dir_all(&antman_path)?;

    global_env::set_global_env("ANTMAN_PATH", antman_path.to_str().unwrap())?;

    let mut config_path = antman_path.clone();
    config_path.push("config.json");

    fs::write(config_path, NORMAL_CONFIG)?;

    let mut mod_path = antman_path.clone();
    mod_path.push("crates");

    fs::create_dir_all(mod_path)?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), io::Error> {
    let args = args::ArgsCli::parse();

    if args.command == Command::Init {
        return init();
    }

    if args.command == Command::Build {
        let result = compile_project_handler().await;
        if let Err(err) = result {
            return Err(io::Error::new(io::ErrorKind::Other, err));
        }
    }

    if let Command::Add { name } = args.command {
        let result = add_crate_handler(name).await;
        if let Err(err) = result {
            return Err(io::Error::new(io::ErrorKind::Other, err));
        }
    } else if let Command::New { name, project_type } = args.command {
        create_project_handler(
            name,
            if project_type.bin { "bin" } else { "lib" }.to_string(),
        )
        .await
        .map_err(|it| io::Error::new(io::ErrorKind::Other, it))?;
    }

    Ok(())
}

pub static ANTMAN_PATH: Lazy<PathBuf> = Lazy::new(|| {
    let v = global_env::get_global_env("ANTMAN_PATH");
    let antman_path = match &v {
        Some(it) => Path::new(it),
        None => {
            panic!(
                "{:?}",
                io::Error::new(io::ErrorKind::NotFound, "cannot found env var ANTMAN_PATH")
            )
        }
    };

    if !antman_path.exists() {
        panic!(
            "{:?}",
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("cannot found path: {antman_path:?}")
            )
        )
    }

    antman_path.to_owned()
});

pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    serde_json::from_str::<Config>(
        &fs::read_to_string(ANTMAN_PATH.clone().join("config.json")).expect(&format!(
            "{:?}",
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("cannot found path: {ANTMAN_PATH:?}")
            )
        )),
    )
    .map_err(|e| {
        panic!(
            "{:?}",
            io::Error::new(
                io::ErrorKind::Unsupported,
                format!("deserialize failed: {e}")
            )
        )
    })
    .unwrap()
});
