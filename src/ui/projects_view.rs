use crate::app::App;
use crate::theme::Theme;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, ListState, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, app: &mut App, theme: &Theme, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .margin(2)
        .constraints([
            Constraint::Percentage(22), // Spaces
            Constraint::Length(3),      // Divider
            Constraint::Percentage(34), // Projects
            Constraint::Length(3),      // Divider
            Constraint::Percentage(44), // Activities
        ])
        .split(area);

    let title_style = |focused: bool| {
        if focused {
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.muted)
        }
    };

    // --- Column 0: Spaces ---
    let space_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(3)])
        .split(cols[0]);
    f.render_widget(
        Paragraph::new(Span::styled(
            format!("Spaces ({})", app.spaces.len()),
            title_style(app.projects_pane_focus == 0),
        )),
        space_chunks[0],
    );

    let space_name_w = (space_chunks[1].width as usize).saturating_sub(7).max(6);
    let mut space_items: Vec<ListItem> = Vec::with_capacity(app.spaces.len() + 2);
    {
        let selected = app.selected_space_idx == 0;
        let bg = if selected && app.projects_pane_focus == 0 {
            theme.highlight
        } else {
            theme.bg
        };
        let marker = if selected { "▸ " } else { "  " };
        space_items.push(ListItem::new(Line::from(vec![
            Span::styled(marker, Style::default().fg(theme.primary).bg(bg)),
            Span::styled(
                format!("{:<width$}", "All", width = space_name_w),
                Style::default()
                    .fg(theme.fg)
                    .bg(bg)
                    .add_modifier(if selected {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    }),
            ),
            Span::styled(
                format!(" {:>3}", app.projects.len()),
                Style::default().fg(theme.muted).bg(bg),
            ),
        ])));
    }
    for (idx, sp) in app.spaces.iter().enumerate() {
        let selected = app.selected_space_idx == idx + 1;
        let bg = if selected && app.projects_pane_focus == 0 {
            theme.highlight
        } else {
            theme.bg
        };
        let marker = if selected { "▸ " } else { "  " };
        let count = app
            .projects
            .iter()
            .filter(|p| {
                p.space
                    .as_deref()
                    .map(|s| s.eq_ignore_ascii_case(sp))
                    .unwrap_or(false)
            })
            .count();
        space_items.push(ListItem::new(Line::from(vec![
            Span::styled(marker, Style::default().fg(theme.primary).bg(bg)),
            Span::styled(
                format!("{:<width$}", trunc(sp, space_name_w), width = space_name_w),
                Style::default().fg(theme.secondary).bg(bg),
            ),
            Span::styled(
                format!(" {:>3}", count),
                Style::default().fg(theme.muted).bg(bg),
            ),
        ])));
    }
    app.click_targets.projects_space_area = space_chunks[1];
    let mut space_state = ListState::default().with_selected(Some(app.selected_space_idx));
    f.render_stateful_widget(List::new(space_items), space_chunks[1], &mut space_state);

    // Divider 1
    let div1: Vec<Line> = (0..cols[1].height)
        .map(|_| Line::from(Span::styled(" │ ", Style::default().fg(theme.border))))
        .collect();
    f.render_widget(Paragraph::new(div1), cols[1]);

    // --- Column 1: Projects ---
    let proj_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(3)])
        .split(cols[2]);
    let proj_scope = app.selected_projects_space().unwrap_or("All");
    f.render_widget(
        Paragraph::new(Span::styled(
            format!(
                "Projects · {} ({})",
                proj_scope,
                app.filtered_projects.len()
            ),
            title_style(app.projects_pane_focus == 1),
        )),
        proj_chunks[0],
    );

    app.click_targets.projects_list_area = proj_chunks[1];
    if app.filtered_projects.is_empty() {
        let hint = if app.projects_pane_focus == 1 {
            "No projects. Press 'a' to add one."
        } else {
            "No projects. Switch here (→) & press 'a'."
        };
        f.render_widget(
            Paragraph::new(Span::styled(hint, Style::default().fg(theme.muted))),
            proj_chunks[1],
        );
    } else {
        let proj_name_w = (proj_chunks[1].width as usize).saturating_sub(9).max(10);
        let items: Vec<ListItem> = app
            .filtered_projects
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let selected = i == app.selected_project_idx;
                let bg = if selected && app.projects_pane_focus == 1 {
                    theme.highlight
                } else {
                    theme.bg
                };
                let marker = if selected { "▸ " } else { "  " };
                let label = if app.selected_space_idx == 0 {
                    match p.space.as_deref().filter(|s| !s.is_empty()) {
                        Some(sp) => format!("{} ({})", p.name, sp),
                        None => p.name.clone(),
                    }
                } else {
                    p.name.clone()
                };
                let line = Line::from(vec![
                    Span::styled(marker, Style::default().fg(theme.primary).bg(bg)),
                    Span::styled(
                        format!(
                            "{:<width$}",
                            trunc(&label, proj_name_w),
                            width = proj_name_w
                        ),
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
        let mut proj_state = ListState::default().with_selected(Some(app.selected_project_idx));
        f.render_stateful_widget(List::new(items), proj_chunks[1], &mut proj_state);
    }

    // Divider 2
    let div2: Vec<Line> = (0..cols[3].height)
        .map(|_| Line::from(Span::styled(" │ ", Style::default().fg(theme.border))))
        .collect();
    f.render_widget(Paragraph::new(div2), cols[3]);

    // --- Column 2: Activities for selected project ---
    let act_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(3)])
        .split(cols[4]);

    let proj_header = app
        .filtered_projects
        .get(app.selected_project_idx)
        .map(|p| match p.space.as_deref().filter(|s| !s.is_empty()) {
            Some(sp) => format!("{} › {}", sp, p.name),
            None => p.name.clone(),
        })
        .unwrap_or_else(|| "—".to_string());
    f.render_widget(
        Paragraph::new(Span::styled(
            format!("Activities · {}", proj_header),
            title_style(app.projects_pane_focus == 2),
        )),
        act_chunks[0],
    );

    app.click_targets.projects_act_area = act_chunks[1];
    if app.project_activities.is_empty() {
        let hint = if app.projects_pane_focus == 2 {
            "No activities. Press 'a' to add one."
        } else {
            "No activities. Switch here (→) & press 'a'."
        };
        f.render_widget(
            Paragraph::new(Span::styled(hint, Style::default().fg(theme.muted))),
            act_chunks[1],
        );
    } else {
        let act_name_w = (act_chunks[1].width as usize).saturating_sub(18).max(12);
        let items: Vec<ListItem> = app
            .project_activities
            .iter()
            .enumerate()
            .map(|(i, a)| {
                let selected = i == app.selected_activity_idx;
                let bg = if selected && app.projects_pane_focus == 2 {
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
                    Span::styled(
                        format!("{:<width$}", trunc(&a.name, act_name_w), width = act_name_w),
                        Style::default().fg(fg).bg(bg),
                    ),
                    Span::styled(
                        format!(
                            " {}/{} ({}m)",
                            a.pomodoros_spent, a.estimated_pomodoros, a.total_focus_mins
                        ),
                        Style::default().fg(theme.muted).bg(bg),
                    ),
                ]);
                ListItem::new(line)
            })
            .collect();
        let mut act_state = ListState::default().with_selected(Some(app.selected_activity_idx));
        f.render_stateful_widget(List::new(items), act_chunks[1], &mut act_state);
    }
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
