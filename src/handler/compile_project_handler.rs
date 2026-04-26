use std::{path::PathBuf, process::Command};

use anyhow::anyhow;

use crate::{
    ANTMAN_PATH,
    crate_manager::{download_deps, install_deps},
    crate_toml_config::{CrateKind, load_crate},
    handler::find_crate_root_path,
    krate::MAIN_TA,
    utils::get_antc_path,
};

pub(crate) async fn compile_project_handler() -> anyhow::Result<()> {
    // 这里需要检查本目录是否有 Antman.toml, 没有的话需要向上查找项目根目录
    let crate_root_dir = std::env::current_dir().map_or(PathBuf::from(String::from("./")), |it| it);
    let crate_root_dir = find_crate_root_path(&crate_root_dir).unwrap_or(crate_root_dir);

    if !crate_root_dir.exists() {
        return Err(anyhow!("{} not exists", crate_root_dir.to_string_lossy()));
    }

    if !crate_root_dir.is_dir() {
        return Err(anyhow!("{} not a folder", crate_root_dir.to_string_lossy()));
    }

    let crate_toml_file = crate_root_dir.join("Antman.toml");

    if !crate_toml_file.exists() {
        return Err(anyhow!("{} not exists", crate_toml_file.to_string_lossy()));
    }

    let krate = load_crate(&crate_toml_file).map_err(|it| it)?;

    if matches!(krate.kind, CrateKind::Lib { .. }) {
        todo!()
    }

    // 这里需要获取主 crate 的 lib.ta/main.ta 路径 然后传参给 antc
    let crate_src_dir = crate_root_dir.join("src");
    let crate_main_file = crate_src_dir.join(MAIN_TA);

    let antc = get_antc_path().ok_or_else(|| anyhow!("antc not found"))?;

    let crates_path = ANTMAN_PATH.join("crates");

    // 一步到位 下载、安装依赖，同时收集版本信息
    let v = download_deps(&krate.dependencies, crates_path.clone()).await?;
    
    // 暂且认为用户只运行 debug 模式
    let target_dir = crate_root_dir.join("target");
    let debug_target_dir = target_dir.join("debug");

    if !debug_target_dir.exists() {
        std::fs::create_dir_all(&debug_target_dir)?;
    }

    // 直接从下载结果中提取版本，避免重复 resolve()
    let mut antc = Command::new(antc);
    for (_zip_opt, dep_info) in &v {
        let crate_ver_str = format!("{}-{}", dep_info.name, dep_info.version);
        antc.arg("--extern-crate")
            .arg(crates_path.join(&dep_info.name).join(crate_ver_str).join("src"));
    }

    // 然后再并发解压
    let install_list: Vec<_> = v
        .iter()
        .filter(|it| it.0.is_some())
        .map(|(p, dep)| {
            (
                p.as_ref().unwrap(),
                format!(r#"{}\{}-{}"#, dep.name, dep.name, dep.version),
            )
        })
        .collect();
    
    install_deps(install_list, crates_path, false).await?;

    antc.arg("--file")
        .arg(crate_main_file)
        .arg("--output")
        .arg(debug_target_dir.join(match krate.kind {
            CrateKind::Package { package, .. } => package.name,
            CrateKind::Lib { lib, .. } => lib.name,
        }))
        .status()?;

    Ok(())
}
