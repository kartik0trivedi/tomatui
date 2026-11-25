use crate::app::App;
use crate::theme::Theme;
use crate::timer::{PomodoroPhase, TimerStatus};
use crate::ui::digits::render_bold_time;
use chrono::{Duration, Local};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

pub fn render(f: &mut Frame, app: &mut App, theme: &Theme, area: Rect) {
    if app.timer.status == TimerStatus::Idle {
        render_opening(f, app, theme, area);
    } else {
        render_active(f, app, theme, area);
    }
}

fn render_opening(f: &mut Frame, app: &mut App, theme: &Theme, area: Rect) {
    let (mins, secs) = app.timer.formatted_time();
    let digit_lines: Vec<Line> = render_bold_time(mins, secs)
        .into_iter()
        .map(|l| Line::from(Span::styled(l, Style::default().fg(theme.fg))))
        .collect();

    let v = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(2),    // Top breathing room
            Constraint::Length(5), // Bold geometric numerals
            Constraint::Length(2), // Gap
            Constraint::Length(1), // Project row
            Constraint::Length(1), // Small gap
            Constraint::Length(1), // Plan row
            Constraint::Length(2), // Gap
            Constraint::Length(1), // Start CTA button
            Constraint::Min(2),    // Bottom breathing room
        ])
        .split(area);

    f.render_widget(
        Paragraph::new(digit_lines).alignment(Alignment::Center),
        v[1],
    );

    let row_w = 48u16.min(area.width.saturating_sub(4));
    let row_x = area.x + (area.width.saturating_sub(row_w)) / 2;

    // Project row
    let proj_rect = Rect::new(row_x, v[3].y, row_w, 1);
    app.click_targets.opening_project_row = proj_rect;
    let proj_bg = if app.opening_focus == 0 {
        theme.highlight
    } else {
        theme.bg
    };
    let ctx_label = app.active_context_label();
    let proj_line = Line::from(vec![
        Span::styled("  Project   ", Style::default().fg(theme.muted).bg(proj_bg)),
        Span::styled(
            format!("{:<29}", truncate(&ctx_label, 29)),
            Style::default()
                .fg(theme.fg)
                .bg(proj_bg)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" (p)  ", Style::default().fg(theme.muted).bg(proj_bg)),
    ]);
    f.render_widget(Paragraph::new(proj_line), proj_rect);

    // Plan row
    let plan_rect = Rect::new(row_x, v[5].y, row_w, 1);
    app.click_targets.opening_plan_row = plan_rect;
    let plan_bg = if app.opening_focus == 1 {
        theme.highlight
    } else {
        theme.bg
    };
    let plan_label = app.timer.plan.label();
    let plan_line = Line::from(vec![
        Span::styled("  Plan      ", Style::default().fg(theme.muted).bg(plan_bg)),
        Span::styled(
            format!("{:<29}", truncate(&plan_label, 29)),
            Style::default()
                .fg(theme.fg)
                .bg(plan_bg)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" (t)  ", Style::default().fg(theme.muted).bg(plan_bg)),
    ]);
    f.render_widget(Paragraph::new(plan_line), plan_rect);

    // Primary Start CTA button
    let btn_text = "   Enter · Start Focus   ";
    let btn_w = btn_text.chars().count() as u16;
    let btn_x = area.x + (area.width.saturating_sub(btn_w)) / 2;
    let btn_rect = Rect::new(btn_x, v[7].y, btn_w, 1);
    app.click_targets.opening_start_btn = btn_rect;
    let btn_style = if app.opening_focus == 2 {
        Style::default()
            .fg(theme.button_fg())
            .bg(theme.primary)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(theme.primary)
            .bg(theme.highlight)
            .add_modifier(Modifier::BOLD)
    };
    f.render_widget(Paragraph::new(Span::styled(btn_text, btn_style)), btn_rect);
}

fn render_active(f: &mut Frame, app: &mut App, theme: &Theme, area: Rect) {
    let phase_color = match app.timer.phase {
        PomodoroPhase::Work => theme.work,
        PomodoroPhase::ShortBreak => theme.short_break,
        PomodoroPhase::LongBreak => theme.long_break,
    };

    let v = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Quiet status strip
            Constraint::Min(1),    // Top breathing room
            Constraint::Length(5), // Bold geometric numerals
            Constraint::Length(2), // Gap
            Constraint::Length(1), // Single clean progress line + batch sequence (no redundant time remaining)
            Constraint::Length(2), // Gap
            Constraint::Length(1), // Project / Activity
            Constraint::Length(1), // Note
            Constraint::Length(2), // Gap
            Constraint::Length(1), // Clickable Pause / Stop buttons
            Constraint::Min(1),    // Bottom breathing room
        ])
        .split(area);

    // Top quiet status strip
    let dot_color = if app.timer.status == TimerStatus::Running {
        theme.success
    } else {
        theme.warning
    };
    let status_word = if app.timer.status == TimerStatus::Running {
        app.timer.phase.label()
    } else {
        "Paused"
    };
    let left_status = Line::from(vec![
        Span::raw("  "),
        Span::styled("● ", Style::default().fg(dot_color)),
        Span::styled(
            format!(
                "{} · Session {} of {}",
                status_word, app.timer.current_batch, app.timer.plan.batches
            ),
            Style::default().fg(theme.muted),
        ),
    ]);
    f.render_widget(Paragraph::new(left_status), v[0]);

    let end_time = (Local::now() + Duration::seconds(app.timer.time_remaining_secs as i64))
        .format("Ends at %H:%M  ")
        .to_string();
    let end_w = end_time.len() as u16;
    if area.width > end_w + 4 {
        f.render_widget(
            Paragraph::new(Span::styled(end_time, Style::default().fg(theme.muted))),
            Rect::new(area.x + area.width - end_w, v[0].y, end_w, 1),
        );
    }

    // Large bold geometric numerals
    let (mins, secs) = app.timer.formatted_time();
    let digit_lines: Vec<Line> = render_bold_time(mins, secs)
        .into_iter()
        .map(|l| Line::from(Span::styled(l, Style::default().fg(theme.fg))))
        .collect();
    f.render_widget(
        Paragraph::new(digit_lines).alignment(Alignment::Center),
        v[2],
    );

    // Smooth sub-cell progress bar (8 fractional steps per character).
    // Full and empty cells both use full-cell backgrounds so filled and track heights match
    // identically in Ghostty and other terminals; the track uses a subtle tint of phase_color.
    let track_color = blend_color(phase_color, theme.button_fg(), 0.28);
    let bar_len = (area.width as usize).saturating_sub(28).clamp(24, 48);
    let sub_steps = bar_len * 8;
    let total_eighths =
        ((app.timer.progress_ratio() * sub_steps as f64).round() as usize).min(sub_steps);
    let full_cells = total_eighths / 8;
    let rem_eighths = total_eighths % 8;
    let partial_glyph = match rem_eighths {
        1 => "▏",
        2 => "▎",
        3 => "▍",
        4 => "▌",
        5 => "▋",
        6 => "▊",
        7 => "▉",
        _ => "",
    };
    let empty_cells = bar_len
        .saturating_sub(full_cells)
        .saturating_sub(if rem_eighths > 0 { 1 } else { 0 });

    let batch_seq = if app.timer.plan.batches > 1 {
        format!(
            "  {}m · {}m · {}m{}",
            app.timer.plan.work_mins,
            app.timer.plan.break_mins,
            app.timer.plan.work_mins,
            if app.timer.plan.batches > 2 {
                format!(" (×{})", app.timer.plan.batches)
            } else {
                String::new()
            }
        )
    } else {
        format!("  {}m", app.timer.plan.work_mins)
    };
    let mut progress_spans = Vec::new();
    if full_cells > 0 {
        progress_spans.push(Span::styled(
            " ".repeat(full_cells),
            Style::default().bg(phase_color),
        ));
    }
    if !partial_glyph.is_empty() {
        progress_spans.push(Span::styled(
            partial_glyph,
            Style::default().fg(phase_color).bg(track_color),
        ));
    }
    if empty_cells > 0 {
        progress_spans.push(Span::styled(
            " ".repeat(empty_cells),
            Style::default().bg(track_color),
        ));
    }
    progress_spans.push(Span::styled(batch_seq, Style::default().fg(theme.muted)));
    let progress_line = Line::from(progress_spans);
    f.render_widget(
        Paragraph::new(progress_line).alignment(Alignment::Center),
        v[4],
    );

    // Active project / activity
    f.render_widget(
        Paragraph::new(Span::styled(
            app.active_context_label(),
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        ))
        .alignment(Alignment::Center),
        v[6],
    );

    // Optional note
    if !app.active_note.is_empty() {
        f.render_widget(
            Paragraph::new(Span::styled(
                format!("\"{}\"", app.active_note),
                Style::default().fg(theme.muted),
            ))
            .alignment(Alignment::Center),
            v[7],
        );
    }

    // Clickable Pause / Stop buttons
    let pause_lbl = if app.timer.status == TimerStatus::Running {
        " Space · Pause "
    } else {
        " Space · Resume "
    };
    let stop_lbl = " Esc · Stop ";
    let gap = 3u16;
    let total_w = pause_lbl.chars().count() as u16 + gap + stop_lbl.chars().count() as u16;
    let start_x = area.x + (area.width.saturating_sub(total_w)) / 2;

    let pause_rect = Rect::new(start_x, v[9].y, pause_lbl.chars().count() as u16, 1);
    let stop_rect = Rect::new(
        start_x + pause_rect.width + gap,
        v[9].y,
        stop_lbl.chars().count() as u16,
        1,
    );
    app.click_targets.active_pause_btn = pause_rect;
    app.click_targets.active_stop_btn = stop_rect;

    f.render_widget(
        Paragraph::new(Span::styled(
            pause_lbl,
            Style::default().fg(theme.fg).bg(theme.highlight),
        )),
        pause_rect,
    );
    f.render_widget(
        Paragraph::new(Span::styled(
            stop_lbl,
            Style::default().fg(theme.muted).bg(theme.highlight),
        )),
        stop_rect,
    );
}

fn blend_color(
    fg: ratatui::style::Color,
    bg: ratatui::style::Color,
    alpha: f32,
) -> ratatui::style::Color {
    if let (ratatui::style::Color::Rgb(fr, fg_c, fb), ratatui::style::Color::Rgb(br, bg_c, bb)) =
        (fg, bg)
    {
        let mix = |a: u8, b: u8| -> u8 {
            ((a as f32) * alpha + (b as f32) * (1.0 - alpha))
                .round()
                .clamp(0.0, 255.0) as u8
        };
        ratatui::style::Color::Rgb(mix(fr, br), mix(fg_c, bg_c), mix(fb, bb))
    } else {
        bg
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}
