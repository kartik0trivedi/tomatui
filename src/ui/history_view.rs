use crate::app::App;
use crate::db::SessionStatus;
use crate::theme::Theme;
use chrono::{DateTime, Local};
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
        .constraints([
            Constraint::Length(2), // Filter bar
            Constraint::Length(1), // Column header
            Constraint::Min(4),    // Rows
        ])
        .split(area);

    let filter_line = Line::from(vec![
        Span::styled(
            if !app.history_unassigned_only {
                "All Sessions"
            } else {
                "All Sessions"
            },
            if !app.history_unassigned_only {
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.muted)
            },
        ),
        Span::styled("    ", Style::default()),
        Span::styled(
            format!("Unassigned ({}) [u]", app.stats_report.unassigned_sessions),
            if app.history_unassigned_only {
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.muted)
            },
        ),
    ]);
    f.render_widget(Paragraph::new(filter_line), chunks[0]);

    let header = Line::from(Span::styled(
        format!(
            "  {:<9} {:<16} {:>5}  {:<10} {:<16} {:<18} {}",
            "ID", "Started", "Dur", "Status", "Project", "Activity", "Notes"
        ),
        Style::default().fg(theme.muted),
    ));
    f.render_widget(Paragraph::new(header), chunks[1]);

    if app.sessions.is_empty() {
        f.render_widget(
            Paragraph::new(Span::styled(
                "  No sessions recorded yet.",
                Style::default().fg(theme.muted),
            )),
            chunks[2],
        );
        return;
    }

    let items: Vec<ListItem> = app
        .sessions
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let selected = i == app.selected_session_idx;
            let bg = if selected { theme.highlight } else { theme.bg };
            let marker = if selected { "▸ " } else { "  " };
            let when = DateTime::parse_from_rfc3339(&s.started_at)
                .map(|dt| dt.with_timezone(&Local).format("%b %d %H:%M").to_string())
                .unwrap_or_else(|_| s.started_at[..s.started_at.len().min(12)].to_string());
            let status_col = match s.status {
                SessionStatus::Completed => theme.success,
                SessionStatus::Partial => theme.warning,
                SessionStatus::Skipped => theme.muted,
                SessionStatus::InProgress => theme.primary,
            };
            let proj_str = s.project_name.as_deref().unwrap_or("unassigned");
            let proj_col = if s.project_name.is_some() {
                theme.primary
            } else {
                theme.warning
            };
            let act_str = s.activity_name.as_deref().unwrap_or("—");

            ListItem::new(Line::from(vec![
                Span::styled(marker, Style::default().fg(theme.primary).bg(bg)),
                Span::styled(
                    format!("{:<9} ", s.short_id()),
                    Style::default().fg(theme.muted).bg(bg),
                ),
                Span::styled(format!("{:<16} ", when), Style::default().fg(theme.fg).bg(bg)),
                Span::styled(
                    format!("{:>4}m  ", s.actual_mins()),
                    Style::default().fg(theme.fg).bg(bg),
                ),
                Span::styled(
                    format!("{:<10} ", s.status.as_str()),
                    Style::default().fg(status_col).bg(bg),
                ),
                Span::styled(
                    format!("{:<16} ", trunc(proj_str, 15)),
                    Style::default().fg(proj_col).bg(bg),
                ),
                Span::styled(
                    format!("{:<18} ", trunc(act_str, 17)),
                    Style::default().fg(theme.fg).bg(bg),
                ),
                Span::styled(
                    trunc(&s.notes, 24),
                    Style::default().fg(theme.muted).bg(bg),
                ),
            ]))
        })
        .collect();

    f.render_widget(List::new(items), chunks[2]);
}

fn trunc(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}
