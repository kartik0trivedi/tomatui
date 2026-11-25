use crate::app::App;
use crate::db::SessionStatus;
use crate::theme::Theme;
use crate::timer::PomodoroPhase;
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
            "All Sessions",
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
            "  {:<9} {:<13} {:<5} {:>4}  {:<10} {:<15} {:<17} {}",
            "ID", "Started", "Type", "Dur", "Status", "Project", "Activity", "Notes"
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

    let fixed_cols = 2 + 10 + 14 + 6 + 6 + 11 + 16 + 18;
    let notes_max = (chunks[2].width as usize)
        .saturating_sub(fixed_cols)
        .max(16);

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
            let (phase_lbl, phase_col) = match s.phase {
                PomodoroPhase::Work => ("work", theme.work),
                PomodoroPhase::ShortBreak => ("rest", theme.short_break),
                PomodoroPhase::LongBreak => ("long", theme.long_break),
            };
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
                Span::styled(
                    format!("{:<13} ", when),
                    Style::default().fg(theme.fg).bg(bg),
                ),
                Span::styled(
                    format!("{:<5} ", phase_lbl),
                    Style::default().fg(phase_col).bg(bg),
                ),
                Span::styled(
                    format!("{:>3}m  ", s.actual_mins()),
                    Style::default().fg(theme.fg).bg(bg),
                ),
                Span::styled(
                    format!("{:<10} ", s.status.as_str()),
                    Style::default().fg(status_col).bg(bg),
                ),
                Span::styled(
                    format!("{:<15} ", trunc(proj_str, 14)),
                    Style::default().fg(proj_col).bg(bg),
                ),
                Span::styled(
                    format!("{:<17} ", trunc(act_str, 16)),
                    Style::default().fg(theme.fg).bg(bg),
                ),
                Span::styled(
                    trunc(&s.notes, notes_max),
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
