use std::{env::current_exe, path::PathBuf};

use global_env::get_global_env;

use crate::CONFIG;

pub fn get_antc_path() -> Option<PathBuf> {
    // 检查 ANTC 环境变量
    if let Some(antc) = get_global_env("ANTC") {
        return Some(antc.into());
    }
    
    // 检查配置
    if let Some(antc) = &CONFIG.antc {
        return Some(antc.into());
    }
    
    // 同目录查找
    let mut exe_path = current_exe().ok()?;
    exe_path.pop(); // 去掉 antman 文件名
    exe_path.push("antc");
    
    if exe_path.exists() {
        return Some(exe_path);
    }

    None
}