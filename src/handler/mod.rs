use std::path::PathBuf;

pub mod add_crate_handler;
pub mod compile_project_handler;
pub mod create_project_handler;

fn find_crate_root_path<P: Into<PathBuf>>(path: P) -> Option<PathBuf> {
    let path = path.into();

    let mut cur_path = path.clone();
    while !cur_path.join("Antman.toml").exists() {
        cur_path = cur_path.parent()?.to_path_buf();
    }

    Some(cur_path)
}