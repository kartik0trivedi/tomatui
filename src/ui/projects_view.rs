use crate::app::App;
use crate::theme::Theme;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .margin(2)
        .constraints([
            Constraint::Percentage(38),
            Constraint::Length(3),
            Constraint::Percentage(62),
        ])
        .split(area);

    // Left: Projects
    let proj_title_style = if app.projects_pane_focus == 0 {
        Style::default()
            .fg(theme.primary)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.muted)
    };
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(3)])
        .split(cols[0]);
    f.render_widget(
        Paragraph::new(Span::styled(
            format!("Projects ({})", app.projects.len()),
            proj_title_style,
        )),
        left_chunks[0],
    );

    if app.projects.is_empty() {
        f.render_widget(
            Paragraph::new(Span::styled(
                "No projects yet. Press 'a' to add one.",
                Style::default().fg(theme.muted),
            )),
            left_chunks[1],
        );
    } else {
        let items: Vec<ListItem> = app
            .projects
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let selected = i == app.selected_project_idx;
                let bg = if selected && app.projects_pane_focus == 0 {
                    theme.highlight
                } else {
                    theme.bg
                };
                let marker = if selected { "▸ " } else { "  " };
                let line = Line::from(vec![
                    Span::styled(marker, Style::default().fg(theme.primary).bg(bg)),
                    Span::styled(
                        format!("{:<20}", p.name),
                        Style::default().fg(theme.fg).bg(bg),
                    ),
                    Span::styled(
                        format!(" {:>4}m", p.total_focus_mins),
                        Style::default().fg(theme.muted).bg(bg),
                    ),
                ]);
                ListItem::new(line)
            })
            .collect();
        f.render_widget(List::new(items), left_chunks[1]);
    }

    // Subtle vertical divider
    let div_lines: Vec<Line> = (0..cols[1].height)
        .map(|_| Line::from(Span::styled(" │ ", Style::default().fg(theme.border))))
        .collect();
    f.render_widget(Paragraph::new(div_lines), cols[1]);

    // Right: Activities for selected project
    let act_title_style = if app.projects_pane_focus == 1 {
        Style::default()
            .fg(theme.primary)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.muted)
    };
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(3)])
        .split(cols[2]);

    let proj_name = app
        .projects
        .get(app.selected_project_idx)
        .map(|p| p.name.as_str())
        .unwrap_or("—");
    f.render_widget(
        Paragraph::new(Span::styled(
            format!("Activities · {}", proj_name),
            act_title_style,
        )),
        right_chunks[0],
    );

    if app.project_activities.is_empty() {
        f.render_widget(
            Paragraph::new(Span::styled(
                "No activities. Switch here (→) and press 'a' to add one.",
                Style::default().fg(theme.muted),
            )),
            right_chunks[1],
        );
    } else {
        let items: Vec<ListItem> = app
            .project_activities
            .iter()
            .enumerate()
            .map(|(i, a)| {
                let selected = i == app.selected_activity_idx;
                let bg = if selected && app.projects_pane_focus == 1 {
                    theme.highlight
                } else {
                    theme.bg
                };
                let check = if a.completed { "[x] " } else { "[ ] " };
                let fg = if a.completed { theme.muted } else { theme.fg };
                let line = Line::from(vec![
                    Span::styled(
                        check,
                        Style::default()
                            .fg(if a.completed {
                                theme.success
                            } else {
                                theme.muted
                            })
                            .bg(bg),
                    ),
                    Span::styled(format!("{:<26}", a.name), Style::default().fg(fg).bg(bg)),
                    Span::styled(
                        format!(
                            " {}/{}  ({}m)",
                            a.pomodoros_spent, a.estimated_pomodoros, a.total_focus_mins
                        ),
                        Style::default().fg(theme.muted).bg(bg),
                    ),
                ]);
                ListItem::new(line)
            })
            .collect();
        f.render_widget(List::new(items), right_chunks[1]);
    }
}
