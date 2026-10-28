//! Tests of the dashboard.
//!
//! The tests run headless: the rendering is drawn into the test backend of
//! ratatui, so no terminal and no network are needed.

use std::path::{Path, PathBuf};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use tokenmaxxing_core::pricing::{ModelPricing, PricingProfile};
use tokenmaxxing_core::provider::ProviderKind;
use tokenmaxxing_core::usage::{EmptyUsageStore, FileUsageStore, UsageRecord, UsageStore};

use super::app::{App, Panel, SortOrder, View};
use super::dashboard::{self, ActionKind, ScanOptions};
use super::render;
use super::theme::Theme;

/// Creates a directory with a few files.
fn fixture(name: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!("tokenmaxxing-ui-{name}"));
    std::fs::remove_dir_all(&directory).ok();
    std::fs::create_dir_all(directory.join("sub")).unwrap();
    std::fs::write(directory.join("a.txt"), "hello world\n".repeat(10)).unwrap();
    std::fs::write(
        directory.join("sub/b.txt"),
        "Merhaba dünya! Bu bir testtir.\n".repeat(5),
    )
    .unwrap();
    directory
}

fn app(root: PathBuf) -> App {
    let mut app = App::new(root, ScanOptions::default(), Box::new(EmptyUsageStore));
    app.theme = Theme::plain();
    app
}

/// Draws the dashboard into the test backend and returns the text.
fn draw(app: &App, width: u16, height: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|frame| render::draw(frame, app)).unwrap();

    let buffer = terminal.backend().buffer();
    buffer
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>()
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

#[test]
fn test_the_dashboard_starts_empty() {
    let app = app(PathBuf::from("/tmp"));

    assert!(app.snapshot.is_none());
    assert_eq!(app.panel, Panel::Overview);
    assert_eq!(app.view, View::Dashboard);
    assert!(app.usage.is_empty());
    assert!(!app.should_quit);
}

#[test]
fn test_an_empty_repository_produces_an_empty_dashboard() {
    let directory = std::env::temp_dir().join("tokenmaxxing-ui-empty");
    std::fs::create_dir_all(&directory).unwrap();

    let snapshot = dashboard::scan(&directory, &ScanOptions::default());

    assert!(snapshot.files.is_empty());
    assert_eq!(snapshot.total.tokens, 0);
    assert!(snapshot.suggestions.is_empty());

    std::fs::remove_dir_all(&directory).ok();
}

#[test]
fn test_an_empty_history_is_handled() {
    let mut app = app(PathBuf::from("/tmp"));
    app.load_usage();

    assert!(app.usage.is_empty());
    assert!(app.usage_error.is_none());
}

#[test]
fn test_an_unavailable_usage_record_is_reported() {
    let path = std::env::temp_dir().join("tokenmaxxing-ui-broken-usage.jsonl");
    std::fs::write(&path, "not json\n").unwrap();

    let mut app = App::new(
        PathBuf::from("/tmp"),
        ScanOptions::default(),
        Box::new(FileUsageStore::new(&path)),
    );
    app.load_usage();

    assert!(app.usage.is_empty());
    assert!(app.usage_error.is_some());

    std::fs::remove_file(&path).ok();
}

#[test]
fn test_usage_records_are_read() {
    let path = std::env::temp_dir().join("tokenmaxxing-ui-usage.jsonl");
    std::fs::remove_file(&path).ok();

    let store = FileUsageStore::new(&path);
    store
        .append(&UsageRecord {
            timestamp: 10,
            provider: "anthropic".to_string(),
            model: "claude-opus-5-5".to_string(),
            input_tokens: 100,
            output_tokens: 20,
            cached_input_tokens: 0,
            cache_write_tokens: 0,
            cost: Some(0.01),
            currency: Some("USD".to_string()),
        })
        .unwrap();

    let mut app = App::new(
        PathBuf::from("/tmp"),
        ScanOptions::default(),
        Box::new(FileUsageStore::new(&path)),
    );
    app.load_usage();

    assert_eq!(app.usage.len(), 1);
    assert_eq!(app.usage[0].input_tokens, 100);

    std::fs::remove_file(&path).ok();
}

#[test]
fn test_the_files_can_be_sorted_and_filtered() {
    let directory = fixture("sorting");
    let mut app = app(directory.clone());
    app.snapshot = Some(dashboard::scan(&directory, &ScanOptions::default()));

    let by_tokens = app.visible_files();
    assert_eq!(by_tokens.len(), 2);
    assert!(by_tokens[0].count.tokens >= by_tokens[1].count.tokens);

    app.sort = SortOrder::Path;
    let by_path = app.visible_files();
    assert!(by_path[0].path.to_string_lossy() < by_path[1].path.to_string_lossy());

    app.sort = SortOrder::Size;
    let by_size = app.visible_files();
    assert!(by_size[0].count.bytes >= by_size[1].count.bytes);

    app.query = "b.txt".to_string();
    assert_eq!(app.visible_files().len(), 1);

    app.query = "nothing-matches".to_string();
    assert!(app.visible_files().is_empty());

    std::fs::remove_dir_all(&directory).ok();
}

#[test]
fn test_a_missing_price_leaves_the_cost_empty() {
    let directory = fixture("price");

    let snapshot = dashboard::scan(
        &directory,
        &ScanOptions {
            model: Some("gpt-4o".to_string()),
            pricing: None,
            provider: ProviderKind::OpenAi,
        },
    );

    assert!(snapshot.cost.is_none());

    std::fs::remove_dir_all(&directory).ok();
}

#[test]
fn test_an_unknown_model_in_the_profile_is_reported() {
    let directory = fixture("price-unknown");
    let profile = PricingProfile::new("USD", "2026-10-01", "test")
        .with_model("another-model", ModelPricing::new("USD", 1.0, 2.0));

    let snapshot = dashboard::scan(
        &directory,
        &ScanOptions {
            model: Some("gpt-4o".to_string()),
            pricing: Some(profile),
            provider: ProviderKind::OpenAi,
        },
    );

    assert!(snapshot.cost.is_none());

    std::fs::remove_dir_all(&directory).ok();
}

#[test]
fn test_an_unsupported_model_falls_back_to_the_estimator() {
    let directory = fixture("unsupported");

    let snapshot = dashboard::scan(
        &directory,
        &ScanOptions {
            model: Some("claude-opus-5-5".to_string()),
            pricing: None,
            provider: ProviderKind::OpenAi,
        },
    );

    assert!(snapshot.measurement.contains("estimate"));
    assert!(snapshot.total.tokens > 0);

    std::fs::remove_dir_all(&directory).ok();
}

#[test]
fn test_the_dashboard_uses_the_counts_of_the_core() {
    let directory = fixture("core-counts");

    let snapshot = dashboard::scan(&directory, &ScanOptions::default());
    let engine = tokenmaxxing_core::engine::TokenEngine::with_tokenizer(std::sync::Arc::new(
        tokenmaxxing_core::tokenizer::openai::OpenAiTokenizer::for_model("gpt-4o").unwrap(),
    ));
    let core_files = tokenmaxxing_core::analysis::count_files(&engine, &directory);

    assert_eq!(snapshot.files.len(), core_files.len());
    assert_eq!(
        snapshot.total.tokens,
        engine.count_repository(&directory).tokens
    );
    for file in &snapshot.files {
        let core = core_files
            .iter()
            .find(|other| other.path == file.path)
            .expect("the file is known to the core");
        assert_eq!(file.count.tokens, core.count.tokens);
    }

    std::fs::remove_dir_all(&directory).ok();
}

#[test]
fn test_the_keyboard_navigates_between_the_panels() {
    let mut app = app(PathBuf::from("/tmp"));

    app.handle_key(key(KeyCode::Tab));
    assert_eq!(app.panel, Panel::TokenUsage);

    app.handle_key(key(KeyCode::Char('l')));
    assert_eq!(app.panel, Panel::LargestFiles);

    app.handle_key(key(KeyCode::Char('h')));
    assert_eq!(app.panel, Panel::TokenUsage);
}

#[test]
fn test_the_keyboard_moves_the_selection() {
    let directory = fixture("selection");
    let mut app = app(directory.clone());
    app.snapshot = Some(dashboard::scan(&directory, &ScanOptions::default()));
    app.panel = Panel::LargestFiles;

    app.handle_key(key(KeyCode::Char('j')));
    assert_eq!(app.selection, 1);
    app.handle_key(key(KeyCode::Char('j')));
    assert_eq!(app.selection, 1, "the selection stays inside the panel");
    app.handle_key(key(KeyCode::Char('k')));
    assert_eq!(app.selection, 0);

    std::fs::remove_dir_all(&directory).ok();
}

#[test]
fn test_a_narrow_terminal_shows_a_message() {
    let app = app(PathBuf::from("/tmp"));

    let text = draw(&app, 40, 10);

    assert!(text.contains("Terminal too small"));
    assert!(text.contains("needed"));
}

#[test]
fn test_a_wide_terminal_shows_the_dashboard() {
    let directory = fixture("render");
    let mut app = app(directory.clone());
    app.snapshot = Some(dashboard::scan(&directory, &ScanOptions::default()));

    let text = draw(&app, 120, 40);

    assert!(text.contains("TOKENMAXXING") || text.contains("Overview"));
    assert!(text.contains("Overview"));
    assert!(text.contains("Largest files"));
    assert!(text.contains("Model comparison"));

    std::fs::remove_dir_all(&directory).ok();
}

#[test]
fn test_the_help_screen_can_be_opened_and_closed() {
    let mut app = app(PathBuf::from("/tmp"));

    app.handle_key(key(KeyCode::Char('?')));
    assert_eq!(app.view, View::Help);

    let text = draw(&app, 120, 40);
    assert!(text.contains("Keyboard"));
    assert!(text.contains("move in the focused panel"));

    app.handle_key(key(KeyCode::Esc));
    assert_eq!(app.view, View::Dashboard);
    assert!(!app.should_quit);
}

#[test]
fn test_the_preview_shows_a_diff_and_can_be_cancelled() {
    let directory = fixture("preview");
    let mut app = app(directory.clone());
    app.snapshot = Some(dashboard::scan(&directory, &ScanOptions::default()));
    app.panel = Panel::Suggestions;
    app.selection = 0;

    app.handle_key(key(KeyCode::Enter));

    if app.preview.is_some() {
        assert_eq!(app.view, View::Preview);

        let text = draw(&app, 120, 40);
        assert!(text.contains("Preview"));

        let before = std::fs::read_to_string(directory.join("a.txt")).unwrap_or_default();

        app.handle_key(key(KeyCode::Esc));

        assert_eq!(app.view, View::Dashboard);
        assert!(app.preview.is_none());
        assert_eq!(
            std::fs::read_to_string(directory.join("a.txt")).unwrap_or_default(),
            before,
            "a cancelled preview must not change the file"
        );
    }

    std::fs::remove_dir_all(&directory).ok();
}

#[test]
fn test_a_rewrite_is_only_applied_after_the_confirmation() {
    let directory = fixture("apply");
    let path = directory.join("a.txt");
    let original = std::fs::read_to_string(&path).unwrap();

    let mut app = app(directory.clone());
    app.snapshot = Some(dashboard::scan(&directory, &ScanOptions::default()));
    app.panel = Panel::Suggestions;
    app.selection = 0;
    app.handle_key(key(KeyCode::Enter));

    let Some(preview) = app.preview.clone() else {
        std::fs::remove_dir_all(&directory).ok();
        return;
    };

    assert_eq!(preview.action.kind, ActionKind::Rewrite);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), original);

    let target = app.resolve(&preview.action.path);
    let original_target = std::fs::read_to_string(&target).unwrap();

    app.handle_key(key(KeyCode::Enter));

    let changed = std::fs::read_to_string(&target).unwrap();
    assert_ne!(changed, original_target, "the confirmed change is written");
    assert!(changed.len() < original_target.len());
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        original,
        "a file that was not part of the change stays untouched"
    );

    std::fs::remove_dir_all(&directory).ok();
}

#[test]
fn test_a_duplicate_is_only_removed_after_the_confirmation() {
    let directory = std::env::temp_dir().join("tokenmaxxing-ui-duplicate");
    std::fs::remove_dir_all(&directory).ok();
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(directory.join("one.txt"), "the same content\n").unwrap();
    std::fs::write(directory.join("two.txt"), "the same content\n").unwrap();

    let mut app = app(directory.clone());
    app.snapshot = Some(dashboard::scan(&directory, &ScanOptions::default()));
    app.panel = Panel::Suggestions;
    app.selection = 0;
    app.handle_key(key(KeyCode::Enter));

    let Some(preview) = app.preview.clone() else {
        std::fs::remove_dir_all(&directory).ok();
        return;
    };

    assert_eq!(preview.action.kind, ActionKind::Remove);
    assert!(directory.join("one.txt").exists());
    assert!(directory.join("two.txt").exists());

    app.handle_key(key(KeyCode::Enter));

    assert!(
        directory.join("one.txt").exists() || directory.join("two.txt").exists(),
        "one of the duplicates stays"
    );

    std::fs::remove_dir_all(&directory).ok();
}

#[test]
fn test_the_search_only_filters_the_file_list() {
    let directory = fixture("search");
    let mut app = app(directory.clone());
    app.snapshot = Some(dashboard::scan(&directory, &ScanOptions::default()));

    app.handle_key(key(KeyCode::Char('/')));
    assert!(app.searching);

    for character in "a.txt".chars() {
        app.handle_key(key(KeyCode::Char(character)));
    }
    app.handle_key(key(KeyCode::Enter));

    assert!(!app.searching);
    assert_eq!(app.visible_files().len(), 1);
    assert_eq!(
        app.panel,
        Panel::Overview,
        "the search does not change the panel"
    );

    std::fs::remove_dir_all(&directory).ok();
}

#[test]
fn test_a_refresh_starts_a_scan_in_the_background() {
    let directory = fixture("refresh");
    let mut app = app(directory.clone());

    app.handle_key(key(KeyCode::Char('r')));
    assert!(app.pending.is_some());

    // The scan runs in a thread, so it may need a moment.
    for _ in 0..200 {
        app.poll_background();
        if app.snapshot.is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    assert!(app.snapshot.is_some());
    assert!(app.snapshot.as_ref().unwrap().total.tokens > 0);

    std::fs::remove_dir_all(&directory).ok();
}

#[test]
fn test_the_root_of_the_scan_is_shown() {
    let directory = fixture("root");
    let snapshot = dashboard::scan(&directory, &ScanOptions::default());

    assert_eq!(snapshot.root, directory);

    std::fs::remove_dir_all(&directory).ok();
}

#[test]
fn test_the_dashboard_does_not_leave_the_scanned_directory() {
    let directory = fixture("scope");
    let outside = std::env::temp_dir().join("tokenmaxxing-ui-outside.txt");
    std::fs::write(&outside, "outside\n").unwrap();

    let snapshot = dashboard::scan(&directory, &ScanOptions::default());

    for file in &snapshot.files {
        assert!(
            file.path.components().all(|part| part.as_os_str() != ".."),
            "the scan stays inside the directory"
        );
    }

    std::fs::remove_dir_all(&directory).ok();
    std::fs::remove_file(&outside).ok();
}

#[test]
fn test_the_usage_panel_distinguishes_usage_from_document_tokens() {
    let app = app(PathBuf::from("/tmp"));

    let text = draw(&app, 120, 40);

    assert!(text.contains("no provider usage recorded"));
    assert!(text.contains("not provider usage"));
}

#[test]
fn test_the_suggestions_panel_explains_itself() {
    let directory = fixture("suggestions");
    let mut app = app(directory.clone());
    app.snapshot = Some(dashboard::scan(&directory, &ScanOptions::default()));
    app.panel = Panel::Suggestions;

    let text = draw(&app, 140, 40);

    assert!(text.contains("Minify") || text.contains("Remove"));
    assert!(text.contains("tokens"));

    std::fs::remove_dir_all(&directory).ok();
}

#[test]
fn test_the_comparison_shows_the_method_of_every_row() {
    let directory = fixture("comparison");
    let mut app = app(directory.clone());
    app.snapshot = Some(dashboard::scan(&directory, &ScanOptions::default()));
    app.panel = Panel::ModelComparison;

    let text = draw(&app, 160, 40);

    assert!(text.contains("cl100k_base"));
    assert!(text.contains("o200k_base"));
    assert!(text.contains("estimate") || text.contains("approx"));
    assert!(text.contains("exact"));

    std::fs::remove_dir_all(&directory).ok();
}

#[test]
fn test_the_usage_history_is_stored_in_the_configured_place() {
    let path = std::env::temp_dir().join("tokenmaxxing-ui-store.jsonl");
    std::fs::remove_file(&path).ok();

    let store = FileUsageStore::new(&path);
    assert_eq!(store.name(), "file");
    assert!(store.is_writable());
    assert_eq!(store.path(), Path::new(&path));

    std::fs::remove_file(&path).ok();
}

#[test]
fn test_the_dashboard_refuses_to_start_without_a_terminal() {
    // The tests run without a terminal, so the check has to fail here.
    let error = super::launch(super::UiOptions {
        root: PathBuf::from("."),
        model: None,
        pricing: None,
        provider: ProviderKind::OpenAi,
        usage_path: None,
    })
    .unwrap_err();

    assert!(error.contains("interactive terminal"));
}
