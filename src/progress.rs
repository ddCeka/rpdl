use {
    indicatif::{MultiProgress, ProgressBar, ProgressStyle},
    macror::BuilderLite,
    std::{sync::Arc, time::Duration},
};

#[derive(Debug, Clone)]
pub enum ProgressStyleConfig {
    Default,
    Detailed,
    Minimal,
    Custom(String),
}

impl ProgressStyleConfig {
    pub fn to_indicatif_style(&self) -> ProgressStyle {
        match self {
            ProgressStyleConfig::Default => ProgressStyle::default_bar()
                .template("[{elapsed_precise}] {bar:16.cyan/blue} {pos}/{len} {msg}")
                .expect("probably invalid template lol")
                .progress_chars("=>-"),
            ProgressStyleConfig::Detailed => ProgressStyle::default_bar()
                .template("[{elapsed_precise}] {bar:16.cyan/blue} {pos}/{len} ({percent}%) {msg} ETA: {eta}")
                .expect("probably invalid template lol")
                .progress_chars("=>-"),
            ProgressStyleConfig::Minimal => ProgressStyle::default_bar()
                .template("{bar:16.cyan/blue} {pos}/{len}")
                .expect("probably invalid template lol")
                .progress_chars("=>-") ,
            ProgressStyleConfig::Custom(template) => ProgressStyle::default_bar()
                .template(template)
                .expect("probably invalid template lol")
                .progress_chars("=>-") 
        }
    }
}

#[derive(Debug, Clone, BuilderLite)]
pub struct ProgressConfig {
    pub enabled: bool,
    pub show_overall: bool,
    pub show_individual: bool,
    pub overall_style: ProgressStyleConfig,
    pub individual_style: ProgressStyleConfig,
}

impl Default for ProgressConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            show_overall: true,
            show_individual: true,
            overall_style: ProgressStyleConfig::Default,
            individual_style: ProgressStyleConfig::Minimal,
        }
    }
}

impl ProgressConfig {
    pub fn new() -> Self {
        Self::default()
    }
}

pub struct ProgressTracker {
    multi: Arc<MultiProgress>,
    overall: Option<Arc<ProgressBar>>,
    config: ProgressConfig,
}

impl ProgressTracker {
    pub fn new(total: u64, config: ProgressConfig) -> Self {
        let multi = Arc::new(MultiProgress::new());

        let overall = if config.enabled && config.show_overall {
            let pb = multi.add(ProgressBar::new(total));

            pb.set_style(config.overall_style.to_indicatif_style());
            pb.set_message("Overall progress");
            pb.tick();

            Some(Arc::new(pb))
        } else {
            None
        };

        Self {
            multi,
            overall,
            config,
        }
    }

    pub fn create_task_progress(&self, id: &str) -> Option<Arc<ProgressBar>> {
        if !self.config.enabled || !self.config.show_individual {
            return None;
        }

        let pb = self.multi.add(ProgressBar::new_spinner());
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} [{elapsed_precise}] {msg}")
                .unwrap(),
        );

        pb.set_message(format!("Downloading {}", id));
        pb.enable_steady_tick(Duration::from_millis(100));

        Some(Arc::new(pb))
    }

    pub fn finish_task(&self, task_pb: Option<Arc<ProgressBar>>, success: bool, message: &str) {
        if let Some(pb) = task_pb {
            if success {
                pb.finish_with_message(format!("Done: {}", message));
            } else {
                pb.finish_with_message(format!("Failed: {}", message));
            }
        }
    }

    pub fn finish(&self) {
        if let Some(overall) = &self.overall {
            overall.finish_with_message("All downloads complete");
        }
    }
}