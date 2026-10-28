//! The state model of the dashboard.
//!
//! The model knows nothing about the terminal. It holds the data of the
//! dashboard, answers key presses and prepares the actions. Rendering reads
//! the model and never changes it, which keeps the two apart and makes the
//! behaviour testable without a terminal.

use std::path::PathBuf;
use std::sync::mpsc::{Receiver, TryRecvError};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tokenmaxxing_core::analysis::FileTokenCount;
use tokenmaxxing_core::usage::{UsageRecord, UsageStore};

use super::dashboard::{self, ActionKind, ProposedAction, ScanOptions, Snapshot, Suggestion};
use super::diff::{diff_lines, DiffLine};
use super::theme::Theme;

/// The panels of the dashboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    /// The summary of the scan.
    Overview,
    /// The token usage of the repository and the history.
    TokenUsage,
    /// The files with the most tokens.
    LargestFiles,
    /// The same text measured with different methods.
    ModelComparison,
    /// What could be optimized.
    Suggestions,
    /// The usage that providers reported.
    ProviderUsage,
}

impl Panel {
    /// Every panel, in the order they are shown.
    pub const ALL: [Panel; 6] = [
        Panel::Overview,
        Panel::TokenUsage,
        Panel::LargestFiles,
        Panel::ModelComparison,
        Panel::Suggestions,
        Panel::ProviderUsage,
    ];

    /// The title of the panel.
    pub fn title(self) -> &'static str {
        match self {
            Panel::Overview => "Overview",
            Panel::TokenUsage => "Token usage",
            Panel::LargestFiles => "Largest files",
            Panel::ModelComparison => "Model comparison",
            Panel::Suggestions => "Optimization suggestions",
            Panel::ProviderUsage => "Provider usage",
        }
    }

    /// The position of the panel in [`Panel::ALL`].
    pub fn index(self) -> usize {
        Panel::ALL
            .iter()
            .position(|panel| *panel == self)
            .unwrap_or(0)
    }
}

/// How the largest files are sorted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    /// By the number of tokens.
    Tokens,
    /// By the size of the file.
    Size,
    /// By the path of the file.
    Path,
}

impl SortOrder {
    /// The label of the sort order.
    pub fn label(self) -> &'static str {
        match self {
            SortOrder::Tokens => "tokens",
            SortOrder::Size => "size",
            SortOrder::Path => "path",
        }
    }

    /// The next sort order.
    pub fn next(self) -> SortOrder {
        match self {
            SortOrder::Tokens => SortOrder::Size,
            SortOrder::Size => SortOrder::Path,
            SortOrder::Path => SortOrder::Tokens,
        }
    }
}

/// What the dashboard shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    /// The panels.
    Dashboard,
    /// The help screen.
    Help,
    /// The preview of a proposed change.
    Preview,
    /// The details of the selected file.
    Detail,
}

/// The kind of a status message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusKind {
    /// Something worked.
    Info,
    /// Something needs attention.
    Warning,
    /// Something failed.
    Error,
}

/// A message in the status bar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Status {
    /// The text of the message.
    pub text: String,
    /// How important the message is.
    pub kind: StatusKind,
}

impl Status {
    /// An informational message.
    pub fn info(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            kind: StatusKind::Info,
        }
    }

    /// A warning.
    pub fn warning(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            kind: StatusKind::Warning,
        }
    }

    /// An error.
    pub fn error(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            kind: StatusKind::Error,
        }
    }
}

/// The preview of a proposed change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preview {
    /// The title of the change.
    pub title: String,
    /// Why the change was proposed.
    pub reason: String,
    /// The change itself.
    pub action: ProposedAction,
    /// The lines of the diff.
    pub diff: Vec<DiffLine>,
}

/// The details of the selected file.
#[derive(Debug, Clone, PartialEq)]
pub struct FileDetail {
    /// The path of the file.
    pub path: PathBuf,
    /// The number of tokens.
    pub tokens: usize,
    /// The size of the file in bytes.
    pub bytes: usize,
    /// The share of the scanned tokens.
    pub share: f64,
    /// The cost, when a profile and a model are known.
    pub cost: Option<f64>,
}

/// The state of the dashboard.
pub struct App {
    /// The scanned directory.
    pub root: PathBuf,
    /// The settings of the scan.
    pub options: ScanOptions,
    /// The palette.
    pub theme: Theme,
    /// The focused panel.
    pub panel: Panel,
    /// The selected item of the focused panel.
    pub selection: usize,
    /// How the largest files are sorted.
    pub sort: SortOrder,
    /// The search query.
    pub query: String,
    /// Whether the search query is being typed.
    pub searching: bool,
    /// What is shown.
    pub view: View,
    /// The preview of a proposed change.
    pub preview: Option<Preview>,
    /// The details of the selected file.
    pub detail: Option<FileDetail>,
    /// The data of the last scan.
    pub snapshot: Option<Snapshot>,
    /// The usage records of the providers.
    pub usage: Vec<UsageRecord>,
    /// Why the usage records could not be read.
    pub usage_error: Option<String>,
    /// The message in the status bar.
    pub status: Option<Status>,
    /// The result of a running scan.
    pub pending: Option<Receiver<Result<Snapshot, String>>>,
    /// Whether the dashboard should close.
    pub should_quit: bool,
    /// The store the usage records come from.
    pub usage_store: Box<dyn UsageStore>,
}

impl App {
    /// Builds the dashboard for a directory.
    pub fn new(root: PathBuf, options: ScanOptions, usage_store: Box<dyn UsageStore>) -> Self {
        Self {
            root,
            options,
            theme: Theme::detect(),
            panel: Panel::Overview,
            selection: 0,
            sort: SortOrder::Tokens,
            query: String::new(),
            searching: false,
            view: View::Dashboard,
            preview: None,
            detail: None,
            snapshot: None,
            usage: Vec::new(),
            usage_error: None,
            status: None,
            pending: None,
            should_quit: false,
            usage_store,
        }
    }

    /// The number of items of the focused panel.
    pub fn item_count(&self) -> usize {
        let snapshot = match &self.snapshot {
            Some(snapshot) => snapshot,
            None => return 0,
        };

        match self.panel {
            Panel::Overview => 0,
            Panel::TokenUsage | Panel::ProviderUsage => self.usage.len(),
            Panel::LargestFiles => self.visible_files().len(),
            Panel::ModelComparison => snapshot.comparison.len(),
            Panel::Suggestions => snapshot.suggestions.len(),
        }
    }

    /// The files of the scan, sorted and filtered.
    pub fn visible_files(&self) -> Vec<&FileTokenCount> {
        let mut files: Vec<&FileTokenCount> = match &self.snapshot {
            Some(snapshot) => snapshot.files.iter().collect(),
            None => Vec::new(),
        };

        match self.sort {
            SortOrder::Tokens => files.sort_by_key(|file| std::cmp::Reverse(file.count.tokens)),
            SortOrder::Size => files.sort_by_key(|file| std::cmp::Reverse(file.count.bytes)),
            SortOrder::Path => files.sort_by_key(|file| file.path.clone()),
        }

        if !self.query.is_empty() {
            let query = self.query.to_lowercase();
            files.retain(|file| file.path.to_string_lossy().to_lowercase().contains(&query));
        }

        files
    }

    /// The suggestions of the scan.
    pub fn suggestions(&self) -> &[Suggestion] {
        match &self.snapshot {
            Some(snapshot) => &snapshot.suggestions,
            None => &[],
        }
    }

    /// Moves the selection of the focused panel.
    pub fn move_selection(&mut self, delta: isize) {
        let count = self.item_count();
        if count == 0 {
            self.selection = 0;
            return;
        }

        let next = self.selection as isize + delta;
        self.selection = next.clamp(0, count as isize - 1) as usize;
    }

    /// Focuses the next panel.
    pub fn next_panel(&mut self) {
        let index = (self.panel.index() + 1) % Panel::ALL.len();
        self.panel = Panel::ALL[index];
        self.selection = 0;
    }

    /// Focuses the previous panel.
    pub fn previous_panel(&mut self) {
        let index = (self.panel.index() + Panel::ALL.len() - 1) % Panel::ALL.len();
        self.panel = Panel::ALL[index];
        self.selection = 0;
    }

    /// Switches to the next sort order.
    pub fn cycle_sort(&mut self) {
        self.sort = self.sort.next();
        self.selection = 0;
        self.status = Some(Status::info(format!("sorted by {}", self.sort.label())));
    }

    /// Starts typing a search query.
    pub fn start_search(&mut self) {
        self.searching = true;
    }

    /// Stops typing a search query.
    pub fn end_search(&mut self) {
        self.searching = false;
        self.selection = 0;
    }

    /// Handles a key press.
    pub fn handle_key(&mut self, key: KeyEvent) {
        if self.searching {
            self.handle_search_key(key);
            return;
        }

        match self.view {
            View::Preview => {
                match key.code {
                    KeyCode::Enter => {
                        let _ = self.apply_preview();
                    }
                    KeyCode::Esc | KeyCode::Char('q') => self.cancel_preview(),
                    _ => {}
                }
                return;
            }
            View::Help | View::Detail => {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') | KeyCode::Enter => self.back(),
                    KeyCode::Char('?') => self.toggle_help(),
                    _ => {}
                }
                return;
            }
            View::Dashboard => {}
        }

        match key.code {
            KeyCode::Char('q') => self.back(),
            KeyCode::Esc => self.back(),
            KeyCode::Char('?') => self.toggle_help(),
            KeyCode::Char('j') | KeyCode::Down => self.move_selection(1),
            KeyCode::Char('k') | KeyCode::Up => self.move_selection(-1),
            KeyCode::Char('l') | KeyCode::Right | KeyCode::Tab => self.next_panel(),
            KeyCode::Char('h') | KeyCode::Left | KeyCode::BackTab => self.previous_panel(),
            KeyCode::Char('/') => self.start_search(),
            KeyCode::Char('r') => self.request_refresh(),
            KeyCode::Char('s') => self.cycle_sort(),
            KeyCode::Enter => self.activate(),
            _ => {}
        }
    }

    /// Handles a key press while the search query is being typed.
    fn handle_search_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.query.clear();
                self.end_search();
            }
            KeyCode::Enter => self.end_search(),
            KeyCode::Backspace => {
                self.query.pop();
                self.selection = 0;
            }
            KeyCode::Char(_) if key.modifiers.contains(KeyModifiers::CONTROL) => {}
            KeyCode::Char(character) => {
                self.query.push(character);
                self.selection = 0;
            }
            _ => {}
        }
    }

    /// Goes back one step, or closes the dashboard.
    pub fn back(&mut self) {
        match self.view {
            View::Dashboard => self.should_quit = true,
            View::Help | View::Preview | View::Detail => {
                self.view = View::Dashboard;
                self.preview = None;
                self.detail = None;
            }
        }
    }

    /// Opens and closes the help screen.
    pub fn toggle_help(&mut self) {
        self.view = match self.view {
            View::Help => View::Dashboard,
            _ => View::Help,
        };
    }

    /// Acts on the selected item of the focused panel.
    pub fn activate(&mut self) {
        match self.panel {
            Panel::Suggestions => self.open_preview(),
            Panel::LargestFiles => self.open_detail(),
            _ => {
                self.status = Some(Status::info("nothing to open here, press ? for the keys"));
            }
        }
    }

    /// Opens the preview of the selected suggestion.
    pub fn open_preview(&mut self) {
        let Some(suggestion) = self.suggestions().get(self.selection).cloned() else {
            return;
        };

        let Some(action) = suggestion.action.clone() else {
            self.status = Some(Status::warning(format!(
                "{} is a hint, the dashboard cannot carry it out on its own",
                suggestion.title
            )));
            return;
        };

        let diff = match action.kind {
            ActionKind::Rewrite => {
                let before =
                    std::fs::read_to_string(self.resolve(&action.path)).unwrap_or_default();
                diff_lines(&before, action.replacement.as_deref().unwrap_or_default())
            }
            ActionKind::Remove => {
                let before =
                    std::fs::read_to_string(self.resolve(&action.path)).unwrap_or_default();
                before
                    .lines()
                    .map(|line| DiffLine::Removed(line.to_string()))
                    .collect()
            }
        };

        self.preview = Some(Preview {
            title: suggestion.title.clone(),
            reason: suggestion.reason.clone(),
            action,
            diff,
        });
        self.view = View::Preview;
    }

    /// Closes the preview without changing anything.
    pub fn cancel_preview(&mut self) {
        self.preview = None;
        self.view = View::Dashboard;
        self.status = Some(Status::info("cancelled, nothing was changed"));
    }

    /// Carries the previewed change out.
    pub fn apply_preview(&mut self) -> Result<(), String> {
        let Some(preview) = self.preview.clone() else {
            return Err("there is no change to apply".to_string());
        };

        let result = match preview.action.kind {
            ActionKind::Rewrite => {
                let replacement = preview.action.replacement.clone().unwrap_or_default();
                let target = self.resolve(&preview.action.path);
                std::fs::write(&target, replacement)
                    .map_err(|error| format!("{}: {error}", target.display()))
            }
            ActionKind::Remove => {
                let keep = self.resolve(&preview.action.keep.clone().unwrap_or_default());
                let removed = self.resolve(&preview.action.path);

                let same = match (
                    std::fs::read_to_string(&keep),
                    std::fs::read_to_string(&removed),
                ) {
                    (Ok(left), Ok(right)) => left == right,
                    _ => false,
                };

                if !same {
                    Err(format!(
                        "{} and {} are not identical any more, nothing was removed",
                        keep.display(),
                        removed.display()
                    ))
                } else {
                    std::fs::remove_file(&removed)
                        .map_err(|error| format!("{}: {error}", removed.display()))
                }
            }
        };

        match result {
            Ok(()) => {
                self.preview = None;
                self.view = View::Dashboard;
                self.status = Some(Status::info(format!("applied: {}", preview.title)));
                self.request_refresh();
                Ok(())
            }
            Err(error) => {
                self.status = Some(Status::error(error.clone()));
                Err(error)
            }
        }
    }

    /// Opens the details of the selected file.
    pub fn open_detail(&mut self) {
        let Some(file) = self.visible_files().get(self.selection).copied() else {
            return;
        };

        let (share, cost) = match &self.snapshot {
            Some(snapshot) => {
                let cost = snapshot
                    .cost
                    .as_ref()
                    .filter(|_| snapshot.total.tokens > 0)
                    .map(|cost| {
                        cost.total * (file.count.tokens as f64 / snapshot.total.tokens as f64)
                    });
                (snapshot.share_of(file), cost)
            }
            None => (0.0, None),
        };

        self.detail = Some(FileDetail {
            path: file.path.clone(),
            tokens: file.count.tokens,
            bytes: file.count.bytes,
            share,
            cost,
        });
        self.view = View::Detail;
    }

    /// Starts a scan in the background.
    pub fn request_refresh(&mut self) {
        let root = self.root.clone();
        let options = self.options.clone();
        let (sender, receiver) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            let _ = sender.send(Ok(dashboard::scan(&root, &options)));
        });

        self.pending = Some(receiver);
        self.status = Some(Status::info(format!(
            "scanning {} ...",
            self.root.display()
        )));
    }

    /// Takes over the result of a running scan.
    pub fn poll_background(&mut self) {
        let Some(receiver) = self.pending.take() else {
            return;
        };

        match receiver.try_recv() {
            Ok(Ok(snapshot)) => {
                self.snapshot = Some(snapshot);
                self.status = Some(Status::info("scan finished"));
            }
            Ok(Err(error)) => {
                self.status = Some(Status::error(format!("the scan failed: {error}")));
            }
            Err(TryRecvError::Empty) => {
                // The scan is still running, keep waiting for it.
                self.pending = Some(receiver);
            }
            Err(TryRecvError::Disconnected) => {
                self.status = Some(Status::error("the scan stopped unexpectedly"));
            }
        }
    }

    /// Resolves a path that is relative to the scanned root.
    pub fn resolve(&self, path: &std::path::Path) -> PathBuf {
        self.root.join(path)
    }

    /// Reads the usage records of the providers.
    pub fn load_usage(&mut self) {
        match self.usage_store.records() {
            Ok(records) => {
                self.usage = records;
                self.usage_error = None;
            }
            Err(error) => {
                self.usage = Vec::new();
                self.usage_error = Some(error.to_string());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokenmaxxing_core::usage::EmptyUsageStore;

    fn app() -> App {
        App::new(
            PathBuf::from("/tmp"),
            ScanOptions::default(),
            Box::new(EmptyUsageStore),
        )
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, crossterm::event::KeyModifiers::NONE)
    }

    #[test]
    fn test_a_new_dashboard_is_empty() {
        let app = app();

        assert_eq!(app.panel, Panel::Overview);
        assert_eq!(app.view, View::Dashboard);
        assert!(app.snapshot.is_none());
        assert!(app.usage.is_empty());
        assert!(!app.should_quit);
    }

    #[test]
    fn test_the_panels_can_be_cycled() {
        let mut app = app();

        app.next_panel();
        assert_eq!(app.panel, Panel::TokenUsage);
        app.next_panel();
        assert_eq!(app.panel, Panel::LargestFiles);
        app.previous_panel();
        assert_eq!(app.panel, Panel::TokenUsage);
        app.previous_panel();
        assert_eq!(app.panel, Panel::Overview);
        app.previous_panel();
        assert_eq!(app.panel, Panel::ProviderUsage);
    }

    #[test]
    fn test_the_selection_stays_inside_the_panel() {
        let mut app = app();

        app.move_selection(5);
        assert_eq!(app.selection, 0);
        app.move_selection(-3);
        assert_eq!(app.selection, 0);
    }

    #[test]
    fn test_quitting_works_from_the_dashboard() {
        let mut app = app();

        app.handle_key(key(KeyCode::Char('q')));

        assert!(app.should_quit);
    }

    #[test]
    fn test_escape_leaves_the_help_screen_without_quitting() {
        let mut app = app();

        app.handle_key(key(KeyCode::Char('?')));
        assert_eq!(app.view, View::Help);

        app.handle_key(key(KeyCode::Esc));
        assert_eq!(app.view, View::Dashboard);
        assert!(!app.should_quit);
    }

    #[test]
    fn test_the_search_query_filters_the_files() {
        let mut app = app();
        app.snapshot = Some(dashboard::scan(
            std::path::Path::new("."),
            &ScanOptions::default(),
        ));

        let all = app.visible_files().len();
        assert!(all > 0);

        app.handle_key(key(KeyCode::Char('/')));
        assert!(app.searching);

        for character in "engine".chars() {
            app.handle_key(key(KeyCode::Char(character)));
        }
        app.handle_key(key(KeyCode::Enter));

        assert!(!app.searching);
        assert!(app.visible_files().len() <= all);
    }

    #[test]
    fn test_the_sort_order_can_be_cycled() {
        let mut app = app();

        assert_eq!(app.sort, SortOrder::Tokens);
        app.cycle_sort();
        assert_eq!(app.sort, SortOrder::Size);
        app.cycle_sort();
        assert_eq!(app.sort, SortOrder::Path);
        app.cycle_sort();
        assert_eq!(app.sort, SortOrder::Tokens);
    }
}
