use clap::{ArgGroup, Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    author = "LKBaka",
    version,
    about = "a package manager for ant",
    long_about = None
)]
pub(crate) struct ArgsCli {
    #[command(subcommand)]
    pub(crate) command: Command,
}

#[derive(Subcommand, Debug, PartialEq, Eq)]
pub(crate) enum Command {
    Init,
    Build,
    Add {
        name: String,
    },
    New {
        name: String,

        #[command(flatten)]
        project_type: ProjectType,
    },
}

#[derive(Args, Debug, PartialEq, Eq)]
#[command(
    group(
        ArgGroup::new("project-type")
            .required(true)
            .args(["bin", "lib"])
    )
)]
pub struct ProjectType {
    #[arg(long)]
    pub bin: bool,

    #[arg(long)]
    pub lib: bool,
}