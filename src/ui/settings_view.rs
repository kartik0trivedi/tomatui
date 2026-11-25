use crate::app::App;
use crate::db::Database;
use crate::theme::Theme;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([Constraint::Length(2), Constraint::Min(10)])
        .split(area);

    f.render_widget(
        Paragraph::new(Span::styled(
            "Settings",
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        )),
        chunks[0],
    );

    let cfg = &app.config;
    let db_display = Database::display_short_path(app.db.path());
    let rows = [
        (
            "Default Focus Duration",
            format!("{} min", cfg.work_duration_mins),
        ),
        (
            "Short Break Duration",
            format!("{} min", cfg.short_break_mins),
        ),
        (
            "Long Break Duration",
            format!("{} min", cfg.long_break_mins),
        ),
        (
            "Long Break Interval",
            format!("Every {} sessions", cfg.long_break_interval),
        ),
        ("Auto-start Breaks", bool_str(cfg.auto_start_breaks)),
        ("Auto-start Work", bool_str(cfg.auto_start_work)),
        ("Audio Chimes", bool_str(cfg.sound_enabled)),
        ("Desktop Notifications", bool_str(cfg.desktop_notifications)),
        (
            "Transparent Background",
            bool_str(cfg.transparent_background),
        ),
        ("Color Theme", format!("◀ {} ▶", cfg.theme.name())),
        (
            "Data Location (SQLite)",
            format!("{}  (Enter to change)", db_display),
        ),
    ];

    let items: Vec<ListItem> = rows
        .iter()
        .enumerate()
        .map(|(i, (label, val))| {
            let selected = i == app.settings_idx;
            let bg = if selected { theme.highlight } else { theme.bg };
            let marker = if selected { "▸ " } else { "  " };
            ListItem::new(Line::from(vec![
                Span::styled(marker, Style::default().fg(theme.primary).bg(bg)),
                Span::styled(
                    format!("{:<26}", label),
                    Style::default().fg(theme.fg).bg(bg),
                ),
                Span::styled(
                    val.as_str(),
                    Style::default()
                        .fg(if selected { theme.primary } else { theme.muted })
                        .bg(bg)
                        .add_modifier(if selected {
                            Modifier::BOLD
                        } else {
                            Modifier::empty()
                        }),
                ),
            ]))
        })
        .collect();

    f.render_widget(List::new(items), chunks[1]);
}

fn bool_str(v: bool) -> String {
    if v {
        "On".to_string()
    } else {
        "Off".to_string()
    }
}
