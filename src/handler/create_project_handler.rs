use std::path::PathBuf;

use anyhow::anyhow;
use toml_edit::{DocumentMut, table};

use crate::krate::{LIB_TA, MAIN_TA};

const MAIN_CONTENT: &str = r#"func main() -> i32 {
    0i32
}"#;

const LIB_CONTENT: &str = r#"func add(a: i32, b: i32) -> i32 {
    a + b
}"#;

const GITIGNORE_CONTENT: &str = r#"/target
.DS_Store
Thumbs.db
desktop.ini"#;

pub(crate) async fn create_project_handler(
    project_name: String,
    project_kind: String,
) -> anyhow::Result<()> {
    if project_kind != "bin" && project_kind != "lib" {
        return Err(anyhow!("invaild project kind: {project_kind}"));
    }

    let is_lib = project_kind == "lib";

    let current_dir = std::env::current_dir().map_or(PathBuf::from(String::from("./")), |it| it);

    let project_dir = current_dir.join(&project_name);

    std::fs::create_dir(&project_dir).map_err(|it| it)?;

    let project_toml_file = project_dir.join(PathBuf::from("Antman.toml"));

    let mut antman_toml = DocumentMut::new();
    antman_toml["package"] = table();
    antman_toml["package"]["name"] = project_name.clone().into();
    antman_toml["package"]["version"] = "0.1.0".into();

    if project_kind == "lib" {
        antman_toml[&project_kind] = table();
        antman_toml[&project_kind]["name"] = project_name.into();
        antman_toml[&project_kind]["version"] = "0.1.0".into();
    }

    std::fs::write(project_toml_file, antman_toml.to_string()).map_err(|it| it)?;

    let src_dir = project_dir.join("src");
    std::fs::create_dir_all(&src_dir).map_err(|it| it)?;

    std::fs::write(
        src_dir.join(if is_lib { LIB_TA } else { MAIN_TA }),
        if is_lib { LIB_CONTENT } else { MAIN_CONTENT },
    )
    .map_err(|it| it)?;

    let gitignore_file = project_dir.join(PathBuf::from(".gitignore"));

    std::fs::write(gitignore_file, GITIGNORE_CONTENT).map_err(|it| it)?;

    Ok(())
}
