use crate::app::App;
use crate::theme::Theme;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{BarChart, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, app: &mut App, theme: &Theme, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(2), // Space filter bar [All, spaces...]
            Constraint::Length(3), // Summary metrics row
            Constraint::Length(1), // Gap
            Constraint::Min(8),    // 7-day chart + project breakdown
        ])
        .split(area);

    // Space filter bar: [All]  Client  Personal  School  Work   (◀ ▶)
    app.click_targets.stats_space_pills.clear();
    let mut filter_spans = vec![Span::styled("Space: ", Style::default().fg(theme.muted))];
    let mut cursor_x = rows[0].x + 7;
    let bar_y = rows[0].y;
    let mut push_pill = |label: &str, idx: usize, active: bool| {
        let text = format!(" {} ", label);
        let w = text.chars().count() as u16;
        app.click_targets
            .stats_space_pills
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
    push_pill("All", 0, app.stats_space_idx == 0);
    for (idx, sp) in app.spaces.iter().enumerate() {
        push_pill(sp, idx + 1, app.stats_space_idx == idx + 1);
    }
    if !app.spaces.is_empty() {
        filter_spans.push(Span::styled("  ←→", Style::default().fg(theme.muted)));
    }
    f.render_widget(Paragraph::new(Line::from(filter_spans)), rows[0]);

    let r = &app.stats_view_report;

    let cards = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(rows[1]);

    let metric = |label: &str, value: String, color| {
        Paragraph::new(vec![
            Line::from(Span::styled(
                label.to_string(),
                Style::default().fg(theme.muted),
            )),
            Line::from(Span::styled(
                value,
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            )),
        ])
    };

    f.render_widget(
        metric(
            "Today",
            format!("{}m ({} sessions)", r.today_focus_mins, r.today_sessions),
            theme.work,
        ),
        cards[0],
    );
    f.render_widget(
        metric(
            "Current Streak",
            format!("{} days", r.current_streak_days),
            theme.warning,
        ),
        cards[1],
    );
    f.render_widget(
        metric(
            "Best Streak",
            format!("{} days", r.best_streak_days),
            theme.success,
        ),
        cards[2],
    );
    f.render_widget(
        metric(
            &format!("Range: {} [r]", r.range.label()),
            format!("{}m ({} sessions)", r.range_focus_mins, r.range_sessions),
            theme.primary,
        ),
        cards[3],
    );

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(52),
            Constraint::Length(3),
            Constraint::Percentage(48),
        ])
        .split(rows[3]);

    // Left: 7-day BarChart
    let chart_data: Vec<(&str, u64)> = r
        .last_7_days
        .iter()
        .map(|b| (b.label.as_str(), b.focus_mins as u64))
        .collect();
    let chart = BarChart::default()
        .data(&chart_data)
        .bar_width(6)
        .bar_gap(1)
        .bar_style(Style::default().fg(theme.primary))
        .value_style(Style::default().fg(theme.fg))
        .label_style(Style::default().fg(theme.muted));
    let left_v = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(5)])
        .split(body[0]);
    f.render_widget(
        Paragraph::new(Span::styled(
            "Last 7 Days (minutes)",
            Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
        )),
        left_v[0],
    );
    f.render_widget(chart, left_v[1]);

    // Right: Project breakdown
    let right_v = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(5)])
        .split(body[2]);
    f.render_widget(
        Paragraph::new(Span::styled(
            format!("By Project ({})", r.range.label()),
            Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
        )),
        right_v[0],
    );

    if r.by_project.is_empty() {
        f.render_widget(
            Paragraph::new(Span::styled(
                "No completed sessions in this range.",
                Style::default().fg(theme.muted),
            )),
            right_v[1],
        );
    } else {
        let max_mins = r
            .by_project
            .iter()
            .map(|p| p.focus_mins)
            .max()
            .unwrap_or(1)
            .max(1);
        let bar_w = (right_v[1].width as usize).saturating_sub(26).clamp(6, 16);
        let lines: Vec<Line> = r
            .by_project
            .iter()
            .take(8)
            .map(|p| {
                let filled =
                    ((p.focus_mins as f64 / max_mins as f64) * bar_w as f64).round() as usize;
                let filled = filled.clamp(1, bar_w);
                let label = if app.stats_space_idx == 0 {
                    match p.space.as_deref().filter(|s| !s.is_empty()) {
                        Some(sp) => format!("{} · {}", sp, p.project),
                        None => p.project.clone(),
                    }
                } else {
                    p.project.clone()
                };
                Line::from(vec![
                    Span::styled(
                        format!("{:<18} ", trunc(&label, 18)),
                        Style::default().fg(theme.fg),
                    ),
                    Span::styled("━".repeat(filled), Style::default().fg(theme.primary)),
                    Span::styled(
                        "━".repeat(bar_w - filled),
                        Style::default().fg(theme.border),
                    ),
                    Span::styled(
                        format!("  {}m", p.focus_mins),
                        Style::default().fg(theme.muted),
                    ),
                ])
            })
            .collect();
        f.render_widget(Paragraph::new(lines), right_v[1]);
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
