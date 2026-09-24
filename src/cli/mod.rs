use {
    crate::cli::{
        args::{Cli, Commands, SpecializedSite},
        completions::generate_completions,
    },
    clap::Parser,
    color_eyre::Result,
};

pub mod args;
pub mod commands;
pub mod completions;
pub mod utils;

pub async fn run() -> Result<()> {
    let cli = Cli::parse();

    if cli.markdown_help {
        clap_markdown::print_help_markdown::<Cli>();
        std::process::exit(0);
    }

    match cli.command {
        Commands::Single {
            url,
            output,
            max_chunks,
            chunk_size,
            strategy,
            no_adaptive,
            no_progress,
            timeout,
        } => {
            commands::single::handle(
                &url,
                &output,
                max_chunks,
                chunk_size,
                strategy,
                !no_adaptive,
                !no_progress && !cli.quiet,
                timeout,
                cli.verbose,
            )
            .await?;
        }

        Commands::Multi {
            urls,
            output_dir,
            max_concurrent,
            unordered,
            no_progress,
            timeout,
            user_agent,
        } => {
            commands::multi::handle(
                urls,
                output_dir,
                max_concurrent,
                unordered,
                !no_progress && !cli.quiet,
                timeout,
                user_agent,
                cli.verbose,
            )
            .await?;
        }

        Commands::Batch {
            file,
            output_dir,
            max_concurrent,
            no_progress,
            timeout,
        } => {
            commands::batch::handle(
                file,
                output_dir,
                max_concurrent,
                !no_progress && !cli.quiet,
                timeout,
                cli.verbose,
            )
            .await?;
        }

        Commands::Info { url, samples } => {
            commands::info::handle(&url, samples, cli.verbose).await?;
        }

        Commands::Resume { file, force } => {
            commands::resume::handle(file, force, cli.verbose).await?;
        }

        Commands::Completions { shell } => {
            generate_completions(shell);
        }

        Commands::Torrent {
            source,
            output_dir,
            select,
            list_only,
            no_progress,
        } => {
            commands::torrent::handle(
                &source,
                output_dir,
                select,
                list_only,
                !no_progress && !cli.quiet,
                cli.verbose,
            )
            .await?;
        }

        Commands::TorrentInfo { source } => {
            commands::torrent::handle_info(&source, cli.verbose).await?;
        }

        Commands::Specialized { site } => match site {
            SpecializedSite::Github {
                repo,
                tag,
                asset,
                output,
                token,
                show_info,
            } => {
                commands::specialized::handle_github(
                    &repo,
                    tag.as_deref(),
                    asset.as_deref(),
                    &output,
                    token,
                    show_info,
                    cli.verbose,
                )
                .await?
            }

            SpecializedSite::List => {
                commands::specialized::list();
            }
        },

        Commands::Schedule { action } => {
            commands::schedule::handle(action, cli.verbose).await?;
        }
    }

    Ok(())
}
