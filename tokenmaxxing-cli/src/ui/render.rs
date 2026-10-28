//! Drawing the dashboard.
//!
//! The rendering only reads the state of the app. Every number it shows comes
//! from the token engine, the repository analysis or the pricing profile, and
//! every number carries the method that produced it.

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{
    Bar, BarChart, Block, BorderType, Cell, List, ListItem, Paragraph, Row, Table, Wrap,
};
use ratatui::Frame;

use tokenmaxxing_core::CountKind;

use super::app::{App, Panel, StatusKind, View};
use super::banner;
use super::diff::DiffLine;
use super::theme::Theme;

/// The smallest terminal the dashboard can be drawn in.
const MIN_WIDTH: u16 = 64;

/// The room the panels need, without the banner and the status bar.
const MIN_PANEL_HEIGHT: u16 = 10;

/// Draws the whole dashboard.
pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let banner_height = banner::height(area.width);

    // The panels need room of their own, the banner takes what is left.
    if area.width < MIN_WIDTH || area.height < banner_height + MIN_PANEL_HEIGHT + 1 {
        draw_too_small(frame, area, app.theme, banner_height);
        return;
    }

    let rows = Layout::vertical([
        Constraint::Length(banner_height),
        Constraint::Min(MIN_PANEL_HEIGHT),
        Constraint::Length(1),
    ])
    .areas::<3>(area);

    draw_banner(frame, rows[0], app);
    draw_grid(frame, rows[1], app);
    draw_status(frame, rows[2], app);

    match app.view {
        View::Help => draw_help(frame, area, app),
        View::Preview => draw_preview(frame, area, app),
        View::Detail => draw_detail(frame, area, app),
        View::Dashboard => {}
    }
}

/// The message for terminals that are too small.
fn draw_too_small(frame: &mut Frame, area: Rect, theme: Theme, banner_height: u16) {
    let needed_height = banner_height + MIN_PANEL_HEIGHT + 1;

    let lines = vec![
        banner::fallback(&theme),
        Line::from(""),
        Line::from(Span::styled(
            "Terminal too small",
            theme.warning().add_modifier(ratatui::style::Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(format!("current:  {}x{}", area.width, area.height)),
        Line::from(format!("needed:  {MIN_WIDTH}x{needed_height}")),
        Line::from(""),
        Line::from("resize the terminal to see the dashboard"),
    ];

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(Block::bordered().border_type(BorderType::Rounded))
            .wrap(Wrap { trim: true }),
        area,
    );
}

/// The banner and the subtitle.
fn draw_banner(frame: &mut Frame, area: Rect, app: &App) {
    let lines = banner::lines(area.width, &app.theme);

    frame.render_widget(Paragraph::new(Text::from(lines)), area);
}

/// The six panels of the dashboard.
fn draw_grid(frame: &mut Frame, area: Rect, app: &App) {
    let rows = Layout::vertical([
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
    ])
    .areas::<3>(area);

    let top =
        Layout::horizontal([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]).areas::<2>(rows[0]);
    let middle =
        Layout::horizontal([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]).areas::<2>(rows[1]);
    let bottom =
        Layout::horizontal([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]).areas::<2>(rows[2]);

    draw_overview(frame, top[0], app);
    draw_token_usage(frame, top[1], app);
    draw_largest_files(frame, middle[0], app);
    draw_model_comparison(frame, middle[1], app);
    draw_suggestions(frame, bottom[0], app);
    draw_provider_usage(frame, bottom[1], app);
}

/// The border of a panel, marked when it is focused.
fn panel_block<'a>(app: &App, panel: Panel, extra: Option<String>) -> Block<'a> {
    let focused = app.panel == panel;
    let border = if focused {
        app.theme.focused()
    } else {
        app.theme.dim()
    };
    let title = if focused {
        app.theme.title()
    } else {
        app.theme.dim()
    };

    let mut label = format!(" {} ", panel.title());
    if let Some(extra) = extra {
        label.push_str("· ");
        label.push_str(&extra);
        label.push(' ');
    }

    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(border)
        .title(Span::styled(label, title))
}

/// The style of a measurement, depending on how trustworthy it is.
fn kind_style(theme: Theme, kind: CountKind) -> Style {
    match kind {
        CountKind::Estimated | CountKind::Approximate => theme.estimate(),
        CountKind::ExactLocal | CountKind::ProviderApi => theme.exact(),
    }
}

/// The overview panel.
fn draw_overview(frame: &mut Frame, area: Rect, app: &App) {
    let theme = app.theme;
    let mut lines: Vec<Line> = Vec::new();

    let Some(snapshot) = &app.snapshot else {
        lines.push(Line::from(Span::styled("scanning ...", theme.dim())));
        frame.render_widget(
            Paragraph::new(Text::from(lines)).block(panel_block(app, Panel::Overview, None)),
            area,
        );
        return;
    };

    lines.push(Line::from(Span::styled(
        snapshot.root.to_string_lossy().to_string(),
        theme.dim(),
    )));
    lines.push(Line::from(vec![
        Span::styled("Files scanned  ", theme.dim()),
        Span::raw(format!("{}", snapshot.files.len())),
    ]));
    if snapshot.skipped > 0 {
        lines.push(Line::from(vec![
            Span::styled("Skipped        ", theme.dim()),
            Span::styled(
                format!("{} (not utf-8 text)", snapshot.skipped),
                theme.warning(),
            ),
        ]));
    }
    lines.push(Line::from(vec![
        Span::styled("Bytes          ", theme.dim()),
        Span::raw(format!("{}", snapshot.total.bytes)),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Characters     ", theme.dim()),
        Span::raw(format!("{}", snapshot.total.chars)),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Tokens         ", theme.dim()),
        Span::styled(
            format!("{}", snapshot.total.tokens),
            kind_style(theme, snapshot.total.kind),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Method         ", theme.dim()),
        Span::raw(snapshot.measurement.clone()),
    ]));

    match &snapshot.cost {
        Some(cost) => lines.push(Line::from(vec![
            Span::styled("Est. cost      ", theme.dim()),
            Span::styled(
                format!("{:.6} {}", cost.total, cost.currency),
                theme.exact(),
            ),
        ])),
        None => lines.push(Line::from(vec![
            Span::styled("Est. cost      ", theme.dim()),
            Span::styled("no pricing profile loaded", theme.dim()),
        ])),
    }

    if !app.usage.is_empty() {
        let input: usize = app.usage.iter().map(|record| record.input_tokens).sum();
        let output: usize = app.usage.iter().map(|record| record.output_tokens).sum();
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("Reported usage ", theme.dim()),
            Span::styled(format!("{input} in / {output} out"), theme.provider()),
        ]));
    }

    frame.render_widget(
        Paragraph::new(Text::from(lines)).block(panel_block(app, Panel::Overview, None)),
        area,
    );
}

/// The token usage panel.
fn draw_token_usage(frame: &mut Frame, area: Rect, app: &App) {
    let theme = app.theme;

    if app.usage.is_empty() {
        let message = match &app.usage_error {
            Some(error) => format!("usage history unavailable: {error}"),
            None => "no usage history yet".to_string(),
        };
        frame.render_widget(
            Paragraph::new(Text::from(vec![
                Line::from(Span::styled(message, theme.dim())),
                Line::from(""),
                Line::from(Span::styled(
                    "document tokens are shown in the overview",
                    theme.dim(),
                )),
            ]))
            .block(panel_block(app, Panel::TokenUsage, None)),
            area,
        );
        return;
    }

    let input: usize = app.usage.iter().map(|record| record.input_tokens).sum();
    let output: usize = app.usage.iter().map(|record| record.output_tokens).sum();

    let lines = vec![
        Line::from(vec![
            Span::styled("input tokens   ", theme.dim()),
            Span::raw(format!("{input}")),
        ]),
        Line::from(vec![
            Span::styled("output tokens  ", theme.dim()),
            Span::raw(format!("{output}")),
        ]),
        Line::from(vec![
            Span::styled("records        ", theme.dim()),
            Span::raw(format!("{}", app.usage.len())),
        ]),
        Line::from(""),
        Line::from(Span::styled("history (input tokens)", theme.dim())),
    ];

    frame.render_widget(
        Paragraph::new(Text::from(lines)).block(panel_block(app, Panel::TokenUsage, None)),
        area,
    );

    let inner = panel_block(app, Panel::TokenUsage, None).inner(area);
    let chart_area = Rect {
        x: inner.x,
        y: inner.y + 5.min(inner.height),
        width: inner.width,
        height: inner.height.saturating_sub(5),
    };

    if chart_area.height > 1 {
        let bars: Vec<Bar> = app
            .usage
            .iter()
            .rev()
            .take(20)
            .map(|record| {
                Bar::default()
                    .value(record.input_tokens as u64)
                    .label(record.model.clone())
                    .style(theme.provider())
            })
            .collect();

        if !bars.is_empty() {
            frame.render_widget(
                BarChart::new(bars)
                    .bar_width(3)
                    .bar_gap(1)
                    .max(app.usage.iter().map(|r| r.input_tokens).max().unwrap_or(1) as u64)
                    .value_style(theme.provider()),
                chart_area,
            );
        }
    }
}

/// The largest files panel.
fn draw_largest_files(frame: &mut Frame, area: Rect, app: &App) {
    let theme = app.theme;
    let files = app.visible_files();

    if files.is_empty() {
        frame.render_widget(
            Paragraph::new(Text::from(Span::styled("no files to show", theme.dim()))).block(
                panel_block(
                    app,
                    Panel::LargestFiles,
                    Some(format!("sort: {}", app.sort.label())),
                ),
            ),
            area,
        );
        return;
    }

    let total = app
        .snapshot
        .as_ref()
        .map(|snapshot| snapshot.total.tokens)
        .unwrap_or(0);
    let cost_total = app
        .snapshot
        .as_ref()
        .and_then(|snapshot| snapshot.cost.as_ref())
        .map(|cost| cost.total);

    let header = Row::new(vec![
        Cell::from("file"),
        Cell::from("tokens"),
        Cell::from("method"),
        Cell::from("share"),
        Cell::from("cost"),
    ]);

    let rows: Vec<Row> = files
        .iter()
        .take(200)
        .map(|file| {
            let share = if total == 0 {
                0.0
            } else {
                file.count.tokens as f64 / total as f64 * 100.0
            };
            let cost = cost_total.map(|cost| {
                if total == 0 {
                    0.0
                } else {
                    cost * (file.count.tokens as f64 / total as f64)
                }
            });

            Row::new(vec![
                Cell::from(file.path.to_string_lossy().to_string()),
                Cell::from(format!("{}", file.count.tokens)),
                Cell::from(file.count.kind.short_label()),
                Cell::from(format!("{share:.1}%")),
                Cell::from(match cost {
                    Some(cost) => format!("{cost:.5}"),
                    None => "-".to_string(),
                }),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(46),
            Constraint::Percentage(12),
            Constraint::Percentage(16),
            Constraint::Percentage(10),
            Constraint::Percentage(16),
        ],
    )
    .header(header)
    .block(panel_block(
        app,
        Panel::LargestFiles,
        Some(format!("sort: {}", app.sort.label())),
    ))
    .row_highlight_style(theme.accent());

    frame.render_widget(table, area);
}

/// The model comparison panel.
fn draw_model_comparison(frame: &mut Frame, area: Rect, app: &App) {
    let theme = app.theme;

    let rows: Vec<Row> = match &app.snapshot {
        Some(snapshot) => snapshot
            .comparison
            .iter()
            .map(|row| {
                Row::new(vec![
                    Cell::from(row.label.clone()),
                    Cell::from(row.model.clone().unwrap_or_else(|| "-".to_string())),
                    Cell::from(row.tokenizer.clone().unwrap_or_else(|| "-".to_string())),
                    Cell::from(format!("{}", row.tokens)),
                    Cell::from(Span::styled(
                        row.kind.short_label(),
                        kind_style(theme, row.kind),
                    )),
                ])
            })
            .collect(),
        None => Vec::new(),
    };

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(26),
            Constraint::Percentage(22),
            Constraint::Percentage(26),
            Constraint::Percentage(12),
            Constraint::Percentage(14),
        ],
    )
    .header(Row::new(vec![
        Cell::from("method"),
        Cell::from("model"),
        Cell::from("tokenizer"),
        Cell::from("tokens"),
        Cell::from("kind"),
    ]))
    .block(panel_block(
        app,
        Panel::ModelComparison,
        Some("same text".to_string()),
    ))
    .row_highlight_style(theme.accent());

    frame.render_widget(table, area);
}

/// The suggestions panel.
fn draw_suggestions(frame: &mut Frame, area: Rect, app: &App) {
    let theme = app.theme;
    let suggestions = app.suggestions();

    if suggestions.is_empty() {
        frame.render_widget(
            Paragraph::new(Text::from(Span::styled("nothing to suggest", theme.dim())))
                .block(panel_block(app, Panel::Suggestions, None)),
            area,
        );
        return;
    }

    let efficiency = app
        .snapshot
        .as_ref()
        .map(|snapshot| snapshot.waste.efficiency_score)
        .unwrap_or(0.0);
    let duplicates = app
        .snapshot
        .as_ref()
        .map(|snapshot| snapshot.waste.duplicates.len())
        .unwrap_or(0);

    let item_count = suggestions.len();
    let items: Vec<ListItem> = suggestions
        .iter()
        .map(|suggestion| {
            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(suggestion.title.clone(), theme.accent()),
                    Span::raw(" "),
                    Span::styled(
                        format!("~{} tokens", suggestion.estimated_tokens),
                        theme.estimate(),
                    ),
                ]),
                Line::from(Span::styled(suggestion.reason.clone(), theme.dim())),
            ])
        })
        .collect();

    let list = List::new(items)
        .block(panel_block(
            app,
            Panel::Suggestions,
            Some(format!(
                "efficiency {efficiency:.0}  duplicates {duplicates}"
            )),
        ))
        .highlight_style(theme.accent());

    let mut state = ratatui::widgets::ListState::default();
    state.select(Some(app.selection.min(item_count.saturating_sub(1))));
    frame.render_stateful_widget(list, area, &mut state);
}

/// The provider usage panel.
fn draw_provider_usage(frame: &mut Frame, area: Rect, app: &App) {
    let theme = app.theme;

    if app.usage.is_empty() {
        let message = match &app.usage_error {
            Some(error) => format!("usage history unavailable: {error}"),
            None => "no provider usage recorded".to_string(),
        };
        frame.render_widget(
            Paragraph::new(Text::from(vec![
                Line::from(Span::styled(message, theme.dim())),
                Line::from(""),
                Line::from(Span::styled(
                    "the token counts of the repository are not provider usage",
                    theme.dim(),
                )),
            ]))
            .block(panel_block(app, Panel::ProviderUsage, None)),
            area,
        );
        return;
    }

    let items: Vec<ListItem> = app
        .usage
        .iter()
        .rev()
        .take(50)
        .map(|record| {
            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(record.provider.clone(), theme.provider()),
                    Span::raw(" "),
                    Span::raw(record.model.clone()),
                ]),
                Line::from(Span::styled(
                    format!("{} in / {} out", record.input_tokens, record.output_tokens),
                    theme.dim(),
                )),
            ])
        })
        .collect();

    frame.render_widget(
        List::new(items).block(panel_block(app, Panel::ProviderUsage, None)),
        area,
    );
}

/// The status bar.
fn draw_status(frame: &mut Frame, area: Rect, app: &App) {
    let theme = app.theme;

    let hint = match app.view {
        View::Dashboard => {
            "j/k move  tab panel  enter open  / search  s sort  r refresh  ? help  q quit"
        }
        View::Help => "q or esc to go back",
        View::Preview => "enter to apply  esc to cancel",
        View::Detail => "q or esc to go back",
    };

    let status = match &app.status {
        Some(status) => {
            let style = match status.kind {
                StatusKind::Info => theme.accent(),
                StatusKind::Warning => theme.warning(),
                StatusKind::Error => theme.warning(),
            };
            Span::styled(status.text.clone(), style)
        }
        None => Span::styled("", theme.dim()),
    };

    let search = if app.searching {
        format!(" /{}", app.query)
    } else if !app.query.is_empty() {
        format!(" filter: {}", app.query)
    } else {
        String::new()
    };

    let line = Line::from(vec![
        status,
        Span::styled(search, theme.accent()),
        Span::raw(" "),
        Span::styled(hint.to_string(), theme.dim()),
    ]);

    frame.render_widget(Paragraph::new(line), area);
}

/// The help screen.
fn draw_help(frame: &mut Frame, area: Rect, app: &App) {
    let theme = app.theme;
    let lines = vec![
        Line::from(Span::styled("Keyboard", theme.title())),
        Line::from(""),
        Line::from(vec![
            Span::styled("j / k, down / up ", theme.accent()),
            Span::raw("move in the focused panel"),
        ]),
        Line::from(vec![
            Span::styled("tab, l / h      ", theme.accent()),
            Span::raw("switch the focused panel"),
        ]),
        Line::from(vec![
            Span::styled("enter           ", theme.accent()),
            Span::raw("open the selection"),
        ]),
        Line::from(vec![
            Span::styled("/                ", theme.accent()),
            Span::raw("search the file list"),
        ]),
        Line::from(vec![
            Span::styled("s                ", theme.accent()),
            Span::raw("change the sort order"),
        ]),
        Line::from(vec![
            Span::styled("r                ", theme.accent()),
            Span::raw("scan again"),
        ]),
        Line::from(vec![
            Span::styled("?                ", theme.accent()),
            Span::raw("open and close this help"),
        ]),
        Line::from(vec![
            Span::styled("q / esc          ", theme.accent()),
            Span::raw("go back, leave the dashboard"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "A change is never applied without an explicit confirmation.",
            theme.dim(),
        )),
    ];

    let width = area.width.saturating_sub(8).min(70);
    let popup = Rect {
        x: area.x + (area.width.saturating_sub(width)) / 2,
        y: area.y + (area.height.saturating_sub(14)) / 2,
        width,
        height: 14.min(area.height),
    };

    frame.render_widget(ratatui::widgets::Clear, popup);
    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .border_style(theme.accent())
                    .title(Span::styled(" Help ", theme.title())),
            )
            .wrap(Wrap { trim: true }),
        popup,
    );
}

/// The preview of a proposed change.
fn draw_preview(frame: &mut Frame, area: Rect, app: &App) {
    let theme = app.theme;

    let Some(preview) = &app.preview else {
        return;
    };

    let mut lines = vec![
        Line::from(Span::styled(preview.title.clone(), theme.title())),
        Line::from(Span::styled(preview.reason.clone(), theme.dim())),
        Line::from(""),
    ];

    for line in preview.diff.iter().take(40) {
        let style = match line {
            DiffLine::Context(_) => theme.dim(),
            DiffLine::Removed(_) => theme.warning(),
            DiffLine::Added(_) => theme.exact(),
        };
        lines.push(Line::from(Span::styled(
            format!("{} {}", line.marker(), line.text()),
            style,
        )));
    }

    if preview.diff.len() > 40 {
        lines.push(Line::from(Span::styled(
            format!("... {} more lines", preview.diff.len() - 40),
            theme.dim(),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "enter applies the change, esc cancels it",
        theme.accent(),
    )));

    let width = area.width.saturating_sub(8).min(100);
    let height = area.height.saturating_sub(4).min(lines.len() as u16 + 4);
    let popup = Rect {
        x: area.x + (area.width.saturating_sub(width)) / 2,
        y: area.y + (area.height.saturating_sub(height)) / 2,
        width,
        height,
    };

    frame.render_widget(ratatui::widgets::Clear, popup);
    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .border_style(theme.warning())
                    .title(Span::styled(" Preview ", theme.title())),
            )
            .wrap(Wrap { trim: false }),
        popup,
    );
}

/// The details of the selected file.
fn draw_detail(frame: &mut Frame, area: Rect, app: &App) {
    let theme = app.theme;

    let Some(detail) = &app.detail else {
        return;
    };

    let lines = vec![
        Line::from(Span::styled(
            detail.path.to_string_lossy().to_string(),
            theme.title(),
        )),
        Line::from(""),
        Line::from(format!("tokens      {}", detail.tokens)),
        Line::from(format!("bytes       {}", detail.bytes)),
        Line::from(format!("share       {:.1}%", detail.share)),
        match detail.cost {
            Some(cost) => Line::from(format!("cost (est.) {cost:.6}")),
            None => Line::from(Span::styled("cost (est.) unknown", theme.dim())),
        },
    ];

    let width = area.width.saturating_sub(8).min(80);
    let popup = Rect {
        x: area.x + (area.width.saturating_sub(width)) / 2,
        y: area.y + (area.height.saturating_sub(10)) / 2,
        width,
        height: 10.min(area.height),
    };

    frame.render_widget(ratatui::widgets::Clear, popup);
    frame.render_widget(
        Paragraph::new(Text::from(lines)).block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .border_style(theme.accent())
                .title(Span::styled(" File ", theme.title())),
        ),
        popup,
    );
}
