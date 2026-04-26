use std::{collections::HashMap, fs};

use anyhow::anyhow;

use crate::{
    ANTMAN_PATH, CONFIG,
    downloader::{Downloader, DownloaderConfig},
    krate::{Crate, crate_version_str},
    unzip::unzip,
};

pub(crate) async fn add_crate_handler(crate_name: String) -> anyhow::Result<()> {
    let crates_path = ANTMAN_PATH.clone().join("crates");

    let crate_base_path = crates_path.join(&crate_name);

    if !crates_path.exists() {
        return Err(anyhow!("{} not exists", crates_path.to_string_lossy()));
    }

    if !crates_path.is_dir() {
        return Err(anyhow!("{} not a folder", crates_path.to_string_lossy()));
    }

    let crate_ver = {
        let client = reqwest::Client::new();

        // 获取索引文件内容
        let response = client
            .get(&CONFIG.mod_index)
            .send()
            .await
            .map_err(|it| it)?;

        let content = response.text().await.map_err(|it| it)?;

        match serde_json::from_str::<HashMap<String, Crate>>(&content) {
            Ok(m) => match m.get(&crate_name) {
                Some(it) => &it
                    .versions
                    .iter()
                    .max()
                    .map_or_else(
                        || Err(anyhow!("no any versions of crate `{crate_name}`")),
                        |it| Ok(it),
                    )?
                    .clone(),
                None => return Err(anyhow!("cannot found crate: {crate_name}")),
            },
            Err(err) => return Err(err.into()),
        }
    };

    let crate_ver_str = crate_version_str(&crate_name, crate_ver);

    let config = DownloaderConfig {
        max_concurrent_downloads: 12,
        user_agent: Some(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36 Edg/140.0.0.0".to_string()
        ),
        ..Default::default()
    };

    let downloader = match Downloader::new(config) {
        Ok(it) => it,
        Err(err) => return Err(err),
    };

    let ziped_module = match downloader
        .download_file(
            &crate_ver.url,
            crate_base_path.join(format!("{crate_ver_str}.zip")),
        )
        .await
    {
        Ok(it) => it,
        Err(err) => return Err(err),
    };

    match unzip(
        &ziped_module,
        &crate_base_path.join(crate_ver_str),
    ) {
        Ok(it) => it,
        Err(err) => return Err(err.into()),
    }

    match fs::remove_file(ziped_module) {
        Ok(_) => Ok(()),
        Err(err) => return Err(err.into()),
    }
}
