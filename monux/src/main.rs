mod commands;
mod parser;

use clap::{Arg, ArgAction, Command, CommandFactory, FromArgMatches};
use commands::{delete, edit, find, init, list, new, rename, rename_dir, sync, tui, version};
use parser::{Cli, Commands};

fn main() -> anyhow::Result<()> {
    let command = localize_command(Cli::command());
    let matches = command.get_matches();
    let cli = Cli::from_arg_matches(&matches)?;

    match cli.command {
        Commands::Version => version::run(),
        Commands::Help => {
            let mut command = localize_command(Cli::command());
            command.print_long_help()?;
            println!();
            return Ok(());
        }
        Commands::Init => init::run()?,
        Commands::New { name, tags, dir } => new::run(name, tags, dir)?,
        Commands::List {
            dir,
            dirs,
            tags,
            path,
        } => list::run(dir, dirs, tags, path)?,
        Commands::Find {
            query,
            tags,
            content,
            dir,
            path,
        } => find::run(query, tags, content, dir, path)?,
        Commands::Delete { query, yes } => delete::run(query, yes)?,
        Commands::Rename { old, new } => rename::run(old, new)?,
        Commands::RenameDir { old, new } => rename_dir::run(old, new)?,
        Commands::Sync { dir } => sync::run(dir)?,
        Commands::Edit { path } => edit::run(path)?,
        Commands::Tui => tui::run()?,
    }

    Ok(())
}

fn localize_command(mut command: Command) -> Command {
    for subcommand in command.get_subcommands_mut() {
        *subcommand = localize_command(subcommand.clone());
    }

    let template = help_template_for(&command);
    command
        .disable_help_flag(true)
        .disable_help_subcommand(true)
        .arg(
            Arg::new("help")
                .short('h')
                .long("help")
                .action(ArgAction::Help)
                .help("Показать это сообщение"),
        )
        .help_template(template)
}

fn help_template_for(command: &Command) -> String {
    let has_positionals = command.get_positionals().any(|arg| !arg.is_hide_set());
    let has_options = command
        .get_arguments()
        .any(|arg| !arg.is_positional() && !arg.is_hide_set() && arg.get_id() != "help");
    let has_subcommands = command
        .get_subcommands()
        .any(|subcommand| !subcommand.is_hide_set());

    let mut sections = Vec::new();

    if has_subcommands {
        sections.push(String::from("Команды:\n{subcommands}"));
    }
    if has_positionals {
        sections.push(String::from("Аргументы:\n{positionals}"));
    }
    if has_options {
        sections.push(String::from("Опции:\n{options}"));
    }

    sections.push(String::from(
        "Справка:\n  -h, --help   Аргументы для полученяи справки",
    ));

    let mut template = String::from("{before-help}{about-with-newline}\nИспользование: {usage}");
    if !sections.is_empty() {
        template.push_str("\n\n");
        template.push_str(&sections.join("\n\n"));
    }
    template.push_str("{after-help}");

    template
}
