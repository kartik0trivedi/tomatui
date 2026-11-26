use crate::app::App;
use crate::db::SessionStatus;
use crate::theme::Theme;
use crate::timer::PomodoroPhase;
use chrono::{DateTime, Local};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, ListState, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, app: &mut App, theme: &Theme, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(2), // Filter bar
            Constraint::Length(1), // Column header
            Constraint::Min(4),    // Rows
        ])
        .split(area);

    // Space filter bar matching Stats: [All] School Work ...  |  Unassigned (N) [u]
    app.click_targets.history_space_pills.clear();
    let mut filter_spans = vec![Span::styled("Space: ", Style::default().fg(theme.muted))];
    let mut cursor_x = chunks[0].x + 7;
    let bar_y = chunks[0].y;

    let mut push_pill = |label: &str, idx: usize, active: bool| {
        let text = format!(" {} ", label);
        let w = text.chars().count() as u16;
        app.click_targets
            .history_space_pills
            .push((Rect::new(cursor_x, bar_y, w, 1), idx));
        cursor_x = cursor_x.saturating_add(w + 1);
        let style = if active {
            Style::default()
                .fg(theme.button_fg())
                .bg(theme.primary)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.muted)
        };
        filter_spans.push(Span::styled(text, style));
        filter_spans.push(Span::raw(" "));
    };

    push_pill("All", 0, !app.history_unassigned_only && app.history_space_idx == 0);
    for (idx, sp) in app.spaces.iter().enumerate() {
        push_pill(
            sp,
            idx + 1,
            !app.history_unassigned_only && app.history_space_idx == idx + 1,
        );
    }

    filter_spans.push(Span::styled("  │  ", Style::default().fg(theme.border)));
    cursor_x = cursor_x.saturating_add(5);
    let unass_text = format!("Unassigned ({}) [u]", app.stats_report.unassigned_sessions);
    let unass_w = unass_text.chars().count() as u16;
    app.click_targets.history_unassigned_btn = Rect::new(cursor_x, bar_y, unass_w, 1);
    filter_spans.push(Span::styled(
        unass_text,
        if app.history_unassigned_only {
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.muted)
        },
    ));
    f.render_widget(Paragraph::new(Line::from(filter_spans)), chunks[0]);

    let header = Line::from(Span::styled(
        format!(
            "  {:<9} {:<13} {:<5} {:>4}  {:<10} {:<12} {:<14} {:<16} {}",
            "ID", "Started", "Type", "Dur", "Status", "Space", "Project", "Activity", "Notes"
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

    let fixed_cols = 2 + 10 + 14 + 6 + 6 + 11 + 13 + 15 + 17;
    let notes_max = (chunks[2].width as usize)
        .saturating_sub(fixed_cols)
        .max(12);

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
            let space_str = s.space.as_deref().unwrap_or("—");
            let space_col = if s.space.is_some() {
                theme.secondary
            } else {
                theme.muted
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
                    format!("{:<12} ", trunc(space_str, 11)),
                    Style::default().fg(space_col).bg(bg),
                ),
                Span::styled(
                    format!("{:<14} ", trunc(proj_str, 13)),
                    Style::default().fg(proj_col).bg(bg),
                ),
                Span::styled(
                    format!("{:<16} ", trunc(act_str, 15)),
                    Style::default().fg(theme.fg).bg(bg),
                ),
                Span::styled(
                    trunc(&s.notes, notes_max),
                    Style::default().fg(theme.muted).bg(bg),
                ),
            ]))
        })
        .collect();

    app.click_targets.history_list_area = chunks[2];
    let mut state = ListState::default().with_selected(Some(app.selected_session_idx));
    f.render_stateful_widget(List::new(items), chunks[2], &mut state);
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
