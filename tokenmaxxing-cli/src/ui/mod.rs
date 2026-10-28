//! The interactive dashboard.
//!
//! The module is only compiled with the `tui` feature, so a build without it
//! stays free of the terminal dependencies.

pub mod app;
pub mod banner;
pub mod dashboard;
pub mod diff;
pub mod events;
pub mod logo;
pub mod render;
pub mod theme;

#[cfg(test)]
mod tests;

use std::io::{self, IsTerminal};
use std::path::PathBuf;

use tokenmaxxing_core::pricing::PricingProfile;
use tokenmaxxing_core::provider::ProviderKind;
use tokenmaxxing_core::usage::{EmptyUsageStore, FileUsageStore, UsageStore};

use app::App;
use dashboard::ScanOptions;

/// The settings of the dashboard.
#[derive(Debug, Clone)]
pub struct UiOptions {
    /// The directory to analyze.
    pub root: PathBuf,
    /// The model the costs refer to.
    pub model: Option<String>,
    /// The pricing profile, when the caller has one.
    pub pricing: Option<PricingProfile>,
    /// The provider the model belongs to.
    pub provider: ProviderKind,
    /// Where the usage history is kept.
    ///
    /// Without a path the platform convention is used, and when the platform
    /// has no place for it the dashboard simply has no history.
    pub usage_path: Option<PathBuf>,
}

/// Starts the dashboard.
pub fn launch(options: UiOptions) -> Result<(), String> {
    if !io::stdout().is_terminal() {
        return Err("the dashboard needs an interactive terminal. \
             Use `tokenmaxxing count`, `tokenmaxxing stats` or \
             `tokenmaxxing dashboard --json` for output that works in a script."
            .to_string());
    }

    let usage_store: Box<dyn UsageStore> = match options
        .usage_path
        .clone()
        .or_else(FileUsageStore::default_path)
    {
        Some(path) => Box::new(FileUsageStore::new(path)),
        None => Box::new(EmptyUsageStore),
    };

    let scan_options = ScanOptions {
        model: options.model.clone(),
        pricing: options.pricing.clone(),
        provider: options.provider,
    };

    let mut app = App::new(options.root.clone(), scan_options, usage_store);
    app.load_usage();
    app.request_refresh();

    events::run(&mut app).map_err(|error| format!("the dashboard failed: {error}"))
}
