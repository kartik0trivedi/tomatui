use crate::app::App;
use crate::theme::Theme;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{BarChart, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let r = &app.stats_report;
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3), // Summary metrics row
            Constraint::Length(1), // Gap
            Constraint::Min(8),    // 7-day chart + project breakdown
        ])
        .split(area);

    let cards = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(rows[0]);

    let metric = |label: &str, value: String, color| {
        Paragraph::new(vec![
            Line::from(Span::styled(label.to_string(), Style::default().fg(theme.muted))),
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
        .split(rows[2]);

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
            Style::default()
                .fg(theme.fg)
                .add_modifier(Modifier::BOLD),
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
            Style::default()
                .fg(theme.fg)
                .add_modifier(Modifier::BOLD),
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
        let lines: Vec<Line> = r
            .by_project
            .iter()
            .take(8)
            .map(|p| {
                let filled = ((p.focus_mins as f64 / max_mins as f64) * 16.0).round() as usize;
                let filled = filled.clamp(1, 16);
                Line::from(vec![
                    Span::styled(format!("{:<16} ", p.project), Style::default().fg(theme.fg)),
                    Span::styled("━".repeat(filled), Style::default().fg(theme.primary)),
                    Span::styled(
                        "━".repeat(16 - filled),
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
