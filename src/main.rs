mod chat;
mod cli;
mod color;
mod config;
mod providers;
mod registry;
mod utils;
mod version;

use std::path::PathBuf;

use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use cli::{chat::chat_cmd, list::list_cmd, ColorMode};
use config::read_config;
use providers::providers::ProviderIdentifier;
use registry::populate::populated_registry;

#[derive(
    Parser, Default, Clone, Copy, ValueEnum, strum_macros::Display, strum_macros::EnumString,
)]
#[strum(serialize_all = "lowercase")]
pub(crate) enum RequestedColorMode {
    #[default]
    Auto,
    On,
    Off,
}

#[derive(Parser)]
#[command(name = version::NAME)]
#[command(
    about = "A general-purpose CLI for chat models",
    author = "Alex <alex@al.exander.io>",
    version = version::VERSION
)]
struct Opts {
    #[arg(help="Use ANSI color", long, default_value_t = RequestedColorMode::default())]
    color: RequestedColorMode,
    #[arg(help = "Specify alternative config path", long)]
    config: Option<PathBuf>,
    #[command(subcommand)]
    command: Option<Commands>,
    #[arg(
        help = "generate shell completion and exit",
        long = "generate",
        exclusive = true
    )]
    generator: Option<clap_complete::aot::Shell>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start a chat
    Chat(ChatOpts),
    /// List available models
    List(ListArgs),
}

#[derive(Parser, Default)]
pub(crate) struct ChatOpts {
    /// Specifies the model to be used during the chat
    #[arg(short, long)]
    model: Option<String>,
    /// Enter interactive mode
    #[arg(short, long)]
    interactive: bool,
    /// Specify the initial prompt
    prompt: Option<String>,
}

/// Possible listings
#[derive(Subcommand)]
pub(crate) enum ListObject {
    /// Registered models
    Models(ListModelArgs),
    /// Providers
    Providers,
}

/// Output formats
#[derive(
    Parser, ValueEnum, Default, Clone, Copy, strum_macros::Display, strum_macros::EnumString,
)]
#[strum(serialize_all = "snake_case")]
pub(crate) enum ListingFormat {
    /// Format the output as a table
    #[default]
    Table,
    /// Format the output as JSON
    Json,
    /// Format the output as a table without a header
    HeaderlessTable,
}

#[derive(Parser)]
pub(crate) struct ListArgs {
    /// Output the listing with the specified format
    #[arg(short, long, default_value_t = ListingFormat::default())]
    format: ListingFormat,
    /// List the specified object
    #[command(subcommand)]
    object: ListObject,
}

#[derive(Parser, Default)]
pub(crate) struct ListModelArgs {
    /// Limit listing to the specified provider
    #[arg(short, long)]
    provider: Option<ProviderIdentifier>,
}

fn hook_panics_with_reporting() {
    let default_hook = std::panic::take_hook();

    std::panic::set_hook(Box::new(move |info| {
        default_hook(info);

        eprintln!("");
        eprintln!("It seems you may have encountered a bug. If you believe something is not functioning correctly, we would greatly appreciate your help in reporting it. If you're using an older version, please consider updating to the latest release.");
        eprintln!("");
        eprintln!("As of this release, you can submit bug reports through the GitHub issue tracker, though this process may change in the future.");
        eprintln!("See: https://github.com/flu0r1ne/crosstalk/issues/new?labels=bug&projects=&template=bug_report.md&title=Encountered%20a%20panic");
    }));
}

#[tokio::main]
async fn main() {
    hook_panics_with_reporting();

    let cli = Opts::parse();

    let color = ColorMode::resolve_auto(cli.color);

    color::configure_color(color);

    let config = read_config(cli.config);

    let registry = populated_registry(&config).await;

    let editor: Option<PathBuf> = config.editor.map(|s| s.into());

    if let Some(generator) = cli.generator {
        let out_dir = "target/completions";
        let _ = std::fs::create_dir_all(out_dir);
        let _ = clap_complete::generate_to(generator, &mut Opts::command(), version::NAME, out_dir);
        println!("Generated completions for {} in {}", generator, out_dir);
        return;
    }

    match &cli.command {
        Some(Commands::Chat(args)) => {
            chat_cmd(
                editor,
                config.keybindings,
                config.default_model,
                registry,
                args,
            )
            .await
        }
        Some(Commands::List(args)) => list_cmd(color, registry, args).await,
        None => {
            chat_cmd(
                editor,
                config.keybindings,
                config.default_model,
                registry,
                &ChatOpts::default(),
            )
            .await
        }
    }
}
