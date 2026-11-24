pub mod digits;
pub mod history_view;
pub mod modals;
pub mod projects_view;
pub mod settings_view;
pub mod stats_view;
pub mod timer_view;

use crate::app::{ActiveTab, App, ModalState};
use crate::theme::Theme;
use crate::timer::TimerStatus;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, app: &mut App) {
    let theme = Theme::from_choice_with_transparency(
        app.config.theme,
        app.config.transparent_background,
    );
    let area = f.area();

    // Fill background (Color::Reset when transparent so Ghostty/terminal opacity shows through)
    if theme.bg != ratatui::style::Color::Reset {
        f.render_widget(Block::default().style(Style::default().bg(theme.bg)), area);
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Clean header + bottom hairline
            Constraint::Min(10),   // Main content
            Constraint::Length(2), // Top hairline + clean footer
        ])
        .split(area);

    render_header(f, app, &theme, chunks[0]);

    match app.active_tab {
        ActiveTab::Timer => timer_view::render(f, app, &theme, chunks[1]),
        ActiveTab::Projects => projects_view::render(f, app, &theme, chunks[1]),
        ActiveTab::History => history_view::render(f, app, &theme, chunks[1]),
        ActiveTab::Stats => stats_view::render(f, app, &theme, chunks[1]),
        ActiveTab::Settings => settings_view::render(f, app, &theme, chunks[1]),
    }

    render_footer(f, app, &theme, chunks[2]);

    if !matches!(app.modal, ModalState::None) {
        modals::render(f, app, &theme, area);
    }
}

fn render_header(f: &mut Frame, app: &mut App, theme: &Theme, area: Rect) {
    let nav_y = area.y;
    let tabs = [
        ("1", "Timer", ActiveTab::Timer),
        ("2", "Projects", ActiveTab::Projects),
        ("3", "History", ActiveTab::History),
        ("4", "Stats", ActiveTab::Stats),
    ];

    let mut spans = vec![
        Span::raw("  "),
        Span::styled(
            "tomatui",
            Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
        ),
        Span::raw("     "),
    ];

    let mut cursor_x = area.x + 14;
    for (num, label, tab) in tabs {
        let is_active = app.active_tab == tab;
        let num_style = if is_active {
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.fg)
        };
        let label_style = if is_active {
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.muted)
        };
        let w = (num.len() + 1 + label.len()) as u16;
        let rect = Rect::new(cursor_x, nav_y, w, 1);
        match tab {
            ActiveTab::Timer => app.click_targets.tab_timer = rect,
            ActiveTab::Projects => app.click_targets.tab_projects = rect,
            ActiveTab::History => app.click_targets.tab_history = rect,
            ActiveTab::Stats => app.click_targets.tab_stats = rect,
            ActiveTab::Settings => {}
        }
        spans.push(Span::styled(format!("{} ", num), num_style));
        spans.push(Span::styled(label, label_style));
        spans.push(Span::raw("   "));
        cursor_x = cursor_x.saturating_add(w + 3);
    }

    // Right-aligned ", Settings"
    let settings_label = ", Settings  ";
    let settings_w = settings_label.len() as u16;
    let right_x = area.x + area.width.saturating_sub(settings_w);
    app.click_targets.tab_settings = Rect::new(right_x, nav_y, settings_w, 1);

    let line = Line::from(spans);
    f.render_widget(Paragraph::new(line), Rect::new(area.x, nav_y, area.width, 1));

    let settings_line = if app.active_tab == ActiveTab::Settings {
        Line::from(Span::styled(
            settings_label,
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        ))
    } else {
        Line::from(vec![
            Span::styled(", ", Style::default().fg(theme.fg)),
            Span::styled("Settings  ", Style::default().fg(theme.muted)),
        ])
    };
    f.render_widget(
        Paragraph::new(settings_line),
        Rect::new(right_x, nav_y, settings_w, 1),
    );

    // Thin hairline divider
    let rule = "─".repeat(area.width.saturating_sub(4) as usize);
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw("  "),
            Span::styled(rule, Style::default().fg(theme.border)),
        ])),
        Rect::new(area.x, nav_y + 1, area.width, 1),
    );
}

fn render_footer(f: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let rule = "─".repeat(area.width.saturating_sub(4) as usize);
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw("  "),
            Span::styled(rule, Style::default().fg(theme.border)),
        ])),
        Rect::new(area.x, area.y, area.width, 1),
    );

    let hints = match app.active_tab {
        ActiveTab::Timer if app.timer.status == TimerStatus::Idle => {
            "↑↓ Move   Enter Start   p Project   t Plan   ? Help   q Quit  "
        }
        ActiveTab::Timer => "Space Pause   s Skip   p Project   n Note   Esc Stop   q Quit  ",
        ActiveTab::Projects => "←→ Pane   a Add   Space Done   Enter Select   d Archive/Del   q Quit  ",
        ActiveTab::History => "e/Enter Edit   u Unassigned   d Delete   ? Help   q Quit  ",
        ActiveTab::Stats => "r Cycle Range   ? Help   q Quit  ",
        ActiveTab::Settings => "1-4 Tabs   Esc Back   ↑↓ Select   ←→/Enter Edit   q Quit  ",
    };

    let footer_y = area.y + 1;
    let hints_w = hints.chars().count() as u16;

    // If a transient status message is active, give it priority so it never collides with hints
    if let Some(ref msg) = app.status_message {
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!("  {}", msg),
                Style::default().fg(theme.primary),
            ))),
            Rect::new(area.x, footer_y, area.width, 1),
        );
        return;
    }

    let left_summary = format!(
        "  Today: {}m ({} sessions) · {}-day streak",
        app.stats_report.today_focus_mins,
        app.stats_report.today_sessions,
        app.stats_report.current_streak_days
    );
    let left_w = left_summary.chars().count() as u16;

    if area.width >= left_w + hints_w + 2 {
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                left_summary,
                Style::default().fg(theme.muted),
            ))),
            Rect::new(area.x, footer_y, left_w, 1),
        );
        let hx = area.x + area.width.saturating_sub(hints_w);
        f.render_widget(
            Paragraph::new(Span::styled(hints, Style::default().fg(theme.muted))),
            Rect::new(hx, footer_y, hints_w, 1),
        );
    } else if area.width >= hints_w + 2 {
        // Compact floating window: center or right-align hints cleanly with zero overlap
        let hx = area.x + (area.width.saturating_sub(hints_w)) / 2;
        f.render_widget(
            Paragraph::new(Span::styled(hints, Style::default().fg(theme.muted))),
            Rect::new(hx, footer_y, hints_w, 1),
        );
    } else {
        f.render_widget(
            Paragraph::new(Span::styled(
                format!("  {}", hints.trim()),
                Style::default().fg(theme.muted),
            )),
            Rect::new(area.x, footer_y, area.width, 1),
        );
    }
}
