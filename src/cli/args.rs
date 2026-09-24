use {
    clap::{Command, Parser, Subcommand, ValueEnum},
    clap_complete::Generator,
    std::{path::PathBuf, time::Duration},
};

use crate::chunked::ChunkStrategy;

#[allow(clippy::enum_variant_names)]
#[derive(Debug, ValueEnum, Clone)]
pub enum Shell {
    Bash,
    Elvish,
    Fish,
    Zsh,
}

impl Generator for Shell {
    fn file_name(&self, name: &str) -> String {
        match self {
            Shell::Bash => clap_complete::Shell::Bash.file_name(name),
            Shell::Elvish => clap_complete::Shell::Elvish.file_name(name),
            Shell::Fish => clap_complete::Shell::Fish.file_name(name),
            Shell::Zsh => clap_complete::Shell::Zsh.file_name(name),
        }
    }

    fn generate(&self, cmd: &Command, buf: &mut dyn std::io::Write) {
        match self {
            Shell::Bash => clap_complete::Shell::Bash.generate(cmd, buf),
            Shell::Elvish => clap_complete::Shell::Elvish.generate(cmd, buf),
            Shell::Fish => clap_complete::Shell::Fish.generate(cmd, buf),
            Shell::Zsh => clap_complete::Shell::Zsh.generate(cmd, buf),
        }
    }
}

#[derive(Parser)]
#[command(version, author)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[arg(short, long, global = true, help = "Enable verbose output")]
    pub verbose: bool,

    #[arg(short, long, global = true, help = "Suppress all output except errors")]
    pub quiet: bool,

    #[arg(long = "genreadme", hide = true)]
    pub markdown_help: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(
        about = "Download a single file with multi-threaded chunks",
        alias = "s"
    )]
    Single {
        #[arg(help = "URL to download")]
        url: String,

        #[arg(short, long, help = "Output file path")]
        output: PathBuf,

        #[arg(
            short = 'c',
            long,
            default_value = "8",
            help = "Maximum concurrent chunks"
        )]
        max_chunks: usize,

        #[arg(
            short = 's',
            long,
            default_value = "5",
            help = "Chunk size in MB",
            value_parser = parse_size_mb
        )]
        chunk_size: u64,

        #[arg(
            long,
            default_value = "smart",
            help = "Download strategy (simple or smart)"
        )]
        strategy: StrategyArg,

        #[arg(long, help = "Disable adaptive chunk sizing")]
        no_adaptive: bool,

        #[arg(long, help = "Disable progress bars")]
        no_progress: bool,

        #[arg(
            short,
            long,
            default_value = "30",
            help = "Timeout in seconds",
            value_parser = parse_duration_secs
        )]
        timeout: Duration,
    },

    #[command(about = "Download multiple files concurrently", alias = "m")]
    Multi {
        #[arg(help = "URLs to download (can be specified multiple times)", num_args = 1..)]
        urls: Vec<String>,

        #[arg(short, long, help = "Output directory for downloaded files")]
        output_dir: Option<PathBuf>,

        #[arg(
            short = 'c',
            long,
            default_value = "10",
            help = "Maximum concurrent downloads"
        )]
        max_concurrent: usize,

        #[arg(long, help = "Download files in unordered mode (faster)")]
        unordered: bool,

        #[arg(long, help = "Disable progress bars")]
        no_progress: bool,

        #[arg(
            short,
            long,
            default_value = "30",
            help = "Timeout in seconds per file",
            value_parser = parse_duration_secs
        )]
        timeout: Duration,

        #[arg(short, long, help = "Custom user agent string")]
        user_agent: Option<String>,
    },

    #[command(about = "Download URLs from a file (one per line)", alias = "b")]
    Batch {
        #[arg(help = "File containing URLs (one per line)")]
        file: PathBuf,

        #[arg(short, long, help = "Output directory for downloaded files")]
        output_dir: Option<PathBuf>,

        #[arg(
            short = 'c',
            long,
            default_value = "10",
            help = "Maximum concurrent downloads"
        )]
        max_concurrent: usize,

        #[arg(long, help = "Disable progress bars")]
        no_progress: bool,

        #[arg(
            short,
            long,
            default_value = "30",
            help = "Timeout in seconds per file",
            value_parser = parse_duration_secs
        )]
        timeout: Duration,
    },

    #[command(
        about = "Download from specialized sites using their APIs",
        alias = "sp"
    )]
    Specialized {
        #[command(subcommand)]
        site: SpecializedSite,
    },

    #[command(about = "Show network metrics and performance info", alias = "i")]
    Info {
        #[arg(help = "URL to test")]
        url: String,

        #[arg(
            short,
            long,
            default_value = "5",
            help = "Number of test chunks to download"
        )]
        samples: usize,
    },

    #[command(about = "Resume an interrupted download", alias = "r")]
    Resume {
        #[arg(help = "Path to the incomplete download file")]
        file: PathBuf,

        #[arg(short, long, help = "Force resume even if state is corrupted")]
        force: bool,
    },

    #[command(about = "Generate shell completions", alias = "comp")]
    Completions {
        #[arg(help = "Shell to generate completions for")]
        shell: Shell,
    },

    #[command(about = "Download from torrent file or magnet link", alias = "t")]
    Torrent {
        #[arg(help = "Torrent file path, URL, or magnet link")]
        source: String,

        #[arg(short, long, help = "Output directory")]
        output_dir: Option<PathBuf>,

        #[arg(long, help = "Select specific files (regex pattern)")]
        select: Option<String>,

        #[arg(long, help = "List files without downloading")]
        list_only: bool,

        #[arg(long, help = "Disable progress display")]
        no_progress: bool,
    },

    #[command(about = "Get information about a torrent", alias = "ti")]
    TorrentInfo {
        #[arg(help = "Torrent file, URL, or magnet link")]
        source: String,
    },

    #[command(about = "Schedule a download", alias = "sched")]
    Schedule {
        #[command(subcommand)]
        action: ScheduleAction,
    },
}

#[derive(Subcommand)]
pub enum SpecializedSite {
    #[command(about = "Download from GitHub release or source", alias = "gh")]
    Github {
        #[arg(help = "owner/repo or owner/repo#commit")]
        repo: String,

        #[arg(help = "Optional: tag name or 'latest'")]
        tag: Option<String>,

        #[arg(help = "Optional: specific asset name")]
        asset: Option<String>,

        #[arg(short, long, help = "Output file or directory path")]
        output: PathBuf,

        #[arg(
            long,
            env = "GITHUB_TOKEN",
            help = "GitHub personal access token (optional)"
        )]
        token: Option<String>,

        #[arg(long, help = "Show release/repository information")]
        show_info: bool,
    },

    #[command(about = "List all supported specialized sites")]
    List,
}

#[derive(Subcommand)]
pub enum ScheduleAction {
    #[command(about = "Add a new scheduled download", alias = "a")]
    Add {
        #[arg(help = "Unique ID for this scheduled download")]
        id: String,

        #[arg(help = "URL to download")]
        url: String,

        #[arg(short, long, help = "Output path")]
        output: PathBuf,

        #[arg(long, help = "Schedule time (RFC3339 format)")]
        at: String,

        #[arg(long, help = "Repeat pattern (daily, weekly, hourly)")]
        repeat: Option<String>,
    },

    #[command(about = "List all scheduled downloads", alias = "l")]
    List,

    #[command(about = "Remove a scheduled download", alias = "rm")]
    Remove {
        #[arg(help = "ID of scheduled download to remove")]
        id: String,
    },

    #[command(about = "Start the scheduler")]
    Start,

    #[command(about = "Enable/disable a scheduled download", alias = "tog")]
    Toggle {
        #[arg(help = "ID of scheduled download")]
        id: String,

        #[arg(long, help = "Enable or disable")]
        enable: bool,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum StrategyArg {
    Simple,
    Smart,
}

impl From<StrategyArg> for ChunkStrategy {
    fn from(arg: StrategyArg) -> Self {
        match arg {
            StrategyArg::Simple => ChunkStrategy::Simple,
            StrategyArg::Smart => ChunkStrategy::Smart,
        }
    }
}

fn parse_size_mb(s: &str) -> Result<u64, String> {
    s.parse::<u64>()
        .map(|mb| mb * 1024 * 1024)
        .map_err(|_| format!("Invalid size: {}", s))
}

fn parse_duration_secs(s: &str) -> Result<Duration, String> {
    s.parse::<u64>()
        .map(Duration::from_secs)
        .map_err(|_| format!("Invalid duration: {}", s))
}
