use {
    crate::{
        cli::args::ScheduleAction,
        error::{DownloadError, Result},
        scheduler::{RepeatPattern, ScheduledDownload, Scheduler},
    },
    chrono::DateTime,
    colored::Colorize,
    std::path::PathBuf,
};

pub async fn handle(action: ScheduleAction, _verbose: bool) -> Result<()> {
    let schedule_file = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("rpdl")
        .join("schedule.json");

    let scheduler = Scheduler::new(schedule_file);
    scheduler.load_from_disk().await?;

    match action {
        ScheduleAction::Add {
            id,
            url,
            output,
            at,
            repeat,
        } => {
            let scheduled_time = DateTime::parse_from_rfc3339(&at)
                .map_err(|e| DownloadError::InvalidUrl(format!("Invalid time format: {}", e)))?
                .with_timezone(&chrono::Utc);

            let repeat_pattern = repeat.as_ref().and_then(|r| match r.as_str() {
                "daily" => Some(RepeatPattern::Daily),
                "weekly" => Some(RepeatPattern::Weekly),
                "hourly" => Some(RepeatPattern::Hourly),
                "monthly" => Some(RepeatPattern::Monthly),
                _ => None,
            });

            let download = ScheduledDownload {
                id: id.clone(),
                url,
                output_path: output,
                scheduled_time,
                repeat: repeat_pattern,
                enabled: true,
                last_run: None,
                next_run: None,
                download_config: None,
            };

            scheduler.add(download).await?;
            println!("{} {}", "✓ Scheduled:".green(), id);
        }

        ScheduleAction::List => {
            let downloads = scheduler.list().await;
            if downloads.is_empty() {
                println!("No scheduled downloads");
            } else {
                println!("\n{}", "Scheduled Downloads:".cyan().bold());
                for dl in downloads {
                    let status = if dl.enabled { "enabled" } else { "disabled" };
                    println!("\n  ID: {}", dl.id.green());
                    println!("  URL: {}", dl.url);
                    println!("  Output: {}", dl.output_path.display());
                    println!("  Status: {}", status);
                    if let Some(next) = dl.next_run {
                        println!("  Next run: {}", next);
                    }
                }
            }
        }

        ScheduleAction::Remove { id } => {
            if scheduler.remove(&id).await? {
                println!("{} {}", "✓ Removed:".green(), id);
            } else {
                println!("{} {}", "✗ Not found:".red(), id);
            }
        }

        ScheduleAction::Start => {
            println!("{}", "Starting scheduler...".cyan());
            scheduler.start().await?;
            println!("{}", "Scheduler running. Press Ctrl+C to stop.".yellow());

            tokio::signal::ctrl_c().await?;
            scheduler.stop().await;
            println!("\n{}", "Scheduler stopped.".yellow());
        }

        ScheduleAction::Toggle { id, enable } => {
            if scheduler.enable(&id, enable).await? {
                let status = if enable { "enabled" } else { "disabled" };
                println!("{} {} {}", "✓".green(), id, status);
            } else {
                println!("{} {}", "✗ Not found:".red(), id);
            }
        }
    }

    Ok(())
}