use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    CONFIG,
    krate::{Crate, CrateVersion},
};

pub type Deps = HashMap<String, DependencyValue>;

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct CrateConfig {
    #[serde(flatten)]
    pub kind: CrateKind,
    #[serde(default)]
    pub dependencies: Deps,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum Source {
    Path(String),
    Normal(String),
    Git(String),
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum DependencyValue {
    Version(String),
    Detailed(DependencyDetail),
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DependencyDetail {
    pub version: Option<String>,
    pub path: Option<String>,
    pub git: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DetailedDependency {
    pub name: String,
    pub version: String,
    pub path: Option<String>,
    pub source: Source,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
#[allow(unused)]
pub enum CrateKind {
    Package { package: Package },
    Lib { lib: Lib },
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub edition: String,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct Lib {
    pub name: String,
    pub version: String,
    pub edition: String,
}

pub fn load_crate(path: &std::path::Path) -> Result<CrateConfig, toml::de::Error> {
    let content = std::fs::read_to_string(path).unwrap();
    toml::from_str(&content)
}

/// 从 mod_index 查询 crate 最新版本信息
pub async fn query_crate_from_index(crate_name: &str) -> anyhow::Result<CrateVersion> {
    let client = reqwest::Client::new();

    let response = anyhow::Context::context(
        client.get(&CONFIG.mod_index).send().await,
        "failed to query mod index",
    )?;

    let content = response.text().await?;

    let crates: HashMap<String, Crate> =
        anyhow::Context::context(serde_json::from_str(&content), "failed to parse mod index")?;

    let crate_info = crates
        .get(crate_name)
        .ok_or_else(|| anyhow::anyhow!("crate {} not found in mod index", crate_name))?;

    let latest = crate_info
        .versions
        .iter()
        .max_by(|a, b| a.version.cmp(&b.version))
        .ok_or_else(|| anyhow::anyhow!("no versions found for crate {}", crate_name))?;

    Ok(latest.clone())
}

impl DependencyValue {
    /// 转换为 DetailedDependency，自动从 mod_index 查询最新版本
    pub async fn resolve(&self, crate_name: &str) -> anyhow::Result<DetailedDependency> {
        match self {
            DependencyValue::Version(v) => {
                // 查询 mod_index 获取完整信息
                let crate_ver = query_crate_from_index(crate_name).await?;

                Ok(DetailedDependency {
                    name: crate_name.to_string(),
                    version: v.clone(),
                    path: None,
                    source: Source::Normal(crate_ver.url),
                })
            }

            DependencyValue::Detailed(d) => {
                // 这里理论需要爬 git 然后查 Antman.toml 不过我懒得写了
                if let Some(git) = &d.git {
                    return Ok(DetailedDependency {
                        name: crate_name.to_string(),
                        version: d.version.clone().unwrap_or_default(),
                        path: d.path.clone(),
                        source: Source::Git(git.clone()),
                    });
                }

                if let Some(path) = &d.path {
                    return Ok(DetailedDependency {
                        name: crate_name.to_string(),
                        version: d.version.clone().unwrap_or_default(),
                        path: Some(path.clone()),
                        source: Source::Path(path.clone()),
                    });
                }

                // 有版本号但没有 path/git，查询 mod_index
                let crate_ver = query_crate_from_index(crate_name).await?;

                Ok(DetailedDependency {
                    name: crate_name.to_string(),
                    version: crate_ver.version.to_string(),
                    path: None,
                    source: Source::Normal(crate_ver.url.to_string()),
                })
            }
        }
    }
}
