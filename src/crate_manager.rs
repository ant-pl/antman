use std::path::{Path, PathBuf};
use std::sync::Arc;

use futures::stream::{self, StreamExt};

use crate::{
    crate_toml_config::{DependencyValue, Deps, DetailedDependency, Source},
    downloader::{Downloader, DownloaderConfig},
    unzip::unzip,
};

pub async fn download_dep<T: AsRef<str>>(
    dep_name: T,
    dep: &DependencyValue,
    target_dir: &PathBuf,
    downloader: &Arc<Downloader>,
) -> Result<(Option<PathBuf>, DetailedDependency), anyhow::Error> {
    let dep_name = dep_name.as_ref();

    let new_dep = dep.resolve(dep_name).await?;

    let DetailedDependency {
        version, source, ..
    } = &new_dep;

    let url = match source {
        Source::Git(it) => it,
        Source::Normal(it) => it,
        Source::Path(_) => return Ok((None, new_dep)),
    };

    let crate_ver_str = format!("{dep_name}-{version}");

    let dep_dir = target_dir.join(&dep_name).join(&crate_ver_str);
    if dep_dir.exists() && dep_dir.join("src").exists() {
        return Ok((None, new_dep));
    }

    let ziped_module = downloader
        .download_file(&url, target_dir.join(format!("{crate_ver_str}.zip")))
        .await?;

    Ok((Some(ziped_module), new_dep))
}

/// 接受一个 Deps 和下载目录, 返回一个含有所有包的压缩包路径的列表
/// 使用 futures::stream::iter 并发下载所有依赖
pub async fn download_deps(
    deps: &Deps,
    target_dir: PathBuf,
) -> Result<Vec<(Option<PathBuf>, DetailedDependency)>, anyhow::Error> {
    let config = DownloaderConfig {
        max_concurrent_downloads: 12,
        user_agent: Some(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36 Edg/140.0.0.0".to_string()
        ),
        ..Default::default()
    };

    let downloader = Arc::new(Downloader::new(config)?);

    let dep_list: Vec<_> = deps
        .iter()
        .map(|(name, dep)| (name.clone(), dep.clone()))
        .collect();

    let download_results: Vec<_> = stream::iter(dep_list)
        .map(|(dep_name, dep)| {
            let target_dir = target_dir.clone();
            let downloader = Arc::clone(&downloader);
            async move { download_dep(dep_name, &dep, &target_dir, &downloader).await }
        })
        .buffer_unordered(8)
        .collect()
        .await;

    // 检查是否有错误，如果有则返回第一个错误
    let mut dep_zip_paths = vec![];
    for r in download_results {
        dep_zip_paths.push(r?);
    }

    Ok(dep_zip_paths)
}

/// deps: Vec<(dep_zip_path, unzip_dir_name)>
/// 并发解压所有依赖包
pub async fn install_deps<P: AsRef<Path>, S: AsRef<str>>(
    deps: Vec<(P, S)>,
    target_dir: PathBuf,
    keep_origin_file: bool,
) -> Result<(), anyhow::Error> {
    let install_tasks: Vec<_> = deps
        .into_iter()
        .map(|(dep_path, unzip_name)| {
            let target_dir = target_dir.clone();
            let dep_path = dep_path.as_ref().to_path_buf();
            let unzip_name = unzip_name.as_ref().to_string();
            let keep_origin = keep_origin_file;

            tokio::task::spawn_blocking(move || {
                let unzip_dir = target_dir.join(&unzip_name);
                unzip(&dep_path, &unzip_dir)?;

                if !keep_origin {
                    std::fs::remove_file(&dep_path)?;
                }

                Ok::<(), anyhow::Error>(())
            })
        })
        .collect();

    // 等待所有解压任务完成
    for task in install_tasks {
        match task.await {
            Ok(Ok(())) => {}
            Ok(Err(err)) => return Err(err),
            Err(err) => return Err(anyhow::anyhow!("task join error: {}", err)),
        }
    }

    Ok(())
}

pub async fn download_and_install_deps(
    deps: &Deps,
    target_dir: PathBuf,
) -> Result<(), anyhow::Error> {
    let v = download_deps(deps, target_dir.clone()).await?;

    let deps = v
        .iter()
        .filter(|it| it.0.is_some())
        .map(|(p, dep)| {
            (
                p.as_ref().unwrap(),
                format!(r#"{}\{}-{}"#, dep.name, dep.name, dep.version),
            )
        })
        .collect::<Vec<_>>();

    install_deps(deps, target_dir, false).await
}
