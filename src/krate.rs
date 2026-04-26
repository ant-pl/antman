pub const LIB_TA: &str = "lib.ta";
pub const MAIN_TA: &str = "main.ta";

use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(transparent)]
pub struct Crate {
    pub versions: Vec<CrateVersion>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct CrateVersion {
    pub version: Version,
    pub url: String,
}

impl PartialOrd for CrateVersion {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.version.partial_cmp(&other.version)
    }
}

impl Ord for CrateVersion {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.version.cmp(&other.version)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version(pub usize, pub usize, pub usize);

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.0, self.1, self.2)
    }
}

pub fn crate_version_str<T: ToString>(name: T, version: &CrateVersion) -> String {
    let name = name.to_string();

    format!("{name}-{}", version.version.to_string())
}