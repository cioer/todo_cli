use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// Output JSON
    #[arg(long, global = true)]
    pub json: bool,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    Add { title: Option<String> },
    Edit { id: String, new_title: String },
    Delete { id: String },
    Done { id: String, message: Option<String> },
    Schedule { id: String, datetime: String },
    Reschedule { id: String, datetime: String },
    List {
        #[command(subcommand)]
        list: ListCommand,
    },
}

#[derive(Subcommand, Debug)]
pub enum ListCommand {
    Today,
    Backlog,
}

