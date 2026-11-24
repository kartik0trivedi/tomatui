use crate::app::{App, ModalState};
use crate::theme::Theme;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    match &app.modal {
        ModalState::None => {}
        ModalState::Help => render_help(f, theme, area),
        ModalState::PlanEditor {
            focus,
            work_buf,
            break_buf,
            batches_buf,
        } => {
            let popup = centered_rect(54, 11, area);
            f.render_widget(Clear, popup);
            let block = modal_block(" Set Plan ", theme);
            let inner = block.inner(popup);
            f.render_widget(block, popup);

            let field = |label: &str, val: &str, active: bool| {
                let style = if active {
                    Style::default()
                        .fg(theme.button_fg())
                        .bg(theme.primary)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.fg).bg(theme.highlight)
                };
                vec![
                    Span::styled(label.to_string(), Style::default().fg(theme.muted)),
                    Span::styled(format!(" {:<4} ", val), style),
                    Span::raw("  "),
                ]
            };

            let mut row = Vec::new();
            row.extend(field("Focus(m):", work_buf, *focus == 0));
            row.extend(field("Break(5):", break_buf, *focus == 1));
            row.extend(field("Batches(1):", batches_buf, *focus == 2));

            let lines = vec![
                Line::from(Span::styled(
                    " Quick presets:  [a] 25m×1   [b] 25m×2   [c] 50m×2",
                    Style::default().fg(theme.muted),
                )),
                Line::raw(""),
                Line::from(Span::styled(
                    " Or enter 3 numbers (Tab / ←→ to move):",
                    Style::default().fg(theme.fg),
                )),
                Line::raw(""),
                Line::from(row),
                Line::raw(""),
                Line::from(Span::styled(
                    " Defaults if blank: break 5m, batches 1",
                    Style::default().fg(theme.muted),
                )),
                Line::from(Span::styled(
                    " Enter Apply   Esc Cancel",
                    Style::default().fg(theme.primary),
                )),
            ];
            f.render_widget(Paragraph::new(lines), inner);
        }
        ModalState::NoteEditor { buf } => {
            let popup = centered_rect(54, 7, area);
            f.render_widget(Clear, popup);
            let block = modal_block(" Session Note ", theme);
            let inner = block.inner(popup);
            f.render_widget(block, popup);
            let lines = vec![
                Line::from(Span::styled(
                    format!(" {}_", buf),
                    Style::default().fg(theme.fg).bg(theme.highlight),
                )),
                Line::raw(""),
                Line::from(Span::styled(
                    " Enter Save   Esc Cancel",
                    Style::default().fg(theme.muted),
                )),
            ];
            f.render_widget(Paragraph::new(lines), inner);
        }
        ModalState::DbPathEditor { buf } => {
            let popup = centered_rect(66, 9, area);
            f.render_widget(Clear, popup);
            let block = modal_block(" Change Data Location (SQLite) ", theme);
            let inner = block.inner(popup);
            f.render_widget(block, popup);
            let lines = vec![
                Line::from(Span::styled(
                    " Enter file or folder path (supports ~/, blank resets default):",
                    Style::default().fg(theme.muted),
                )),
                Line::raw(""),
                Line::from(Span::styled(
                    format!(" {}_", buf),
                    Style::default().fg(theme.fg).bg(theme.highlight),
                )),
                Line::raw(""),
                Line::from(Span::styled(
                    " Copies existing DB if target doesn't exist yet.",
                    Style::default().fg(theme.muted),
                )),
                Line::from(Span::styled(
                    " Enter Save & Switch   Esc Cancel",
                    Style::default().fg(theme.primary),
                )),
            ];
            f.render_widget(Paragraph::new(lines), inner);
        }
        ModalState::AddItem {
            is_activity,
            name_buf,
            est_buf,
            focus,
        } => {
            let title = if *is_activity {
                " Add Activity "
            } else {
                " Add Project "
            };
            let popup = centered_rect(52, if *is_activity { 9 } else { 7 }, area);
            f.render_widget(Clear, popup);
            let block = modal_block(title, theme);
            let inner = block.inner(popup);
            f.render_widget(block, popup);

            let mut lines = vec![
                Line::from(vec![
                    Span::styled(" Name: ", Style::default().fg(theme.muted)),
                    Span::styled(
                        format!(" {}_ ", name_buf),
                        if *focus == 0 {
                            Style::default().fg(theme.fg).bg(theme.highlight)
                        } else {
                            Style::default().fg(theme.fg)
                        },
                    ),
                ]),
            ];
            if *is_activity {
                lines.push(Line::raw(""));
                lines.push(Line::from(vec![
                    Span::styled(" Est. Pomodoros: ", Style::default().fg(theme.muted)),
                    Span::styled(
                        format!(" {} ", est_buf),
                        if *focus == 1 {
                            Style::default().fg(theme.button_fg()).bg(theme.primary)
                        } else {
                            Style::default().fg(theme.fg).bg(theme.highlight)
                        },
                    ),
                ]));
            }
            lines.push(Line::raw(""));
            lines.push(Line::from(Span::styled(
                " Enter Save   Esc Cancel",
                Style::default().fg(theme.muted),
            )));
            f.render_widget(Paragraph::new(lines), inner);
        }
        ModalState::ContextPicker {
            session_id,
            focus,
            project_input,
            activity_input,
            status,
            notes_input,
            ..
        } => {
            let is_history = session_id.is_some();
            let title = if is_history {
                " Edit Session "
            } else {
                " Select Project & Activity "
            };
            let popup = centered_rect(58, if is_history { 13 } else { 10 }, area);
            f.render_widget(Clear, popup);
            let block = modal_block(title, theme);
            let inner = block.inner(popup);
            f.render_widget(block, popup);

            let input_style = |active: bool| {
                if active {
                    Style::default()
                        .fg(theme.fg)
                        .bg(theme.highlight)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.fg)
                }
            };

            let mut lines = vec![
                Line::from(vec![
                    Span::styled(" Project  : ", Style::default().fg(theme.muted)),
                    Span::styled(
                        format!(
                            " {}{} ",
                            if project_input.is_empty() {
                                "(unassigned)"
                            } else {
                                project_input
                            },
                            if *focus == 0 { "_" } else { "" }
                        ),
                        input_style(*focus == 0),
                    ),
                ]),
                Line::from(Span::styled(
                    "            ↑↓ cycle existing or type new name",
                    Style::default().fg(theme.muted),
                )),
                Line::raw(""),
                Line::from(vec![
                    Span::styled(" Activity : ", Style::default().fg(theme.muted)),
                    Span::styled(
                        format!(
                            " {}{} ",
                            if activity_input.is_empty() {
                                "(none)"
                            } else {
                                activity_input
                            },
                            if *focus == 1 { "_" } else { "" }
                        ),
                        input_style(*focus == 1),
                    ),
                ]),
            ];

            if is_history {
                lines.push(Line::raw(""));
                lines.push(Line::from(vec![
                    Span::styled(" Status   : ", Style::default().fg(theme.muted)),
                    Span::styled(
                        format!(" ◀ {} ▶ ", status.as_str()),
                        input_style(*focus == 2),
                    ),
                ]));
                lines.push(Line::raw(""));
                lines.push(Line::from(vec![
                    Span::styled(" Notes    : ", Style::default().fg(theme.muted)),
                    Span::styled(
                        format!(" {}{} ", notes_input, if *focus == 3 { "_" } else { "" }),
                        input_style(*focus == 3),
                    ),
                ]));
            }

            lines.push(Line::raw(""));
            lines.push(Line::from(Span::styled(
                " Tab Next Field   Enter Save   Esc Cancel",
                Style::default().fg(theme.primary),
            )));
            f.render_widget(Paragraph::new(lines), inner);
        }
    }
}

fn render_help(f: &mut Frame, theme: &Theme, area: Rect) {
    let popup = centered_rect(58, 16, area);
    f.render_widget(Clear, popup);
    let block = modal_block(" Keyboard & Mouse Shortcuts ", theme);
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let lines = vec![
        Line::from(" 1-4 / T P H S / Tab   Jump to Timer, Projects, History, Stats"),
        Line::from(" ,  /  Esc             Open Settings / Return from Settings"),
        Line::raw(""),
        Line::from(" Timer:"),
        Line::from("   Enter / Space     Start, Pause, or Resume"),
        Line::from("   p                 Assign or change Project / Activity"),
        Line::from("   t                 Set Plan (presets or 3 numbers: focus, break, batches)"),
        Line::from("   n / s / Esc       Edit note / Skip phase / Stop"),
        Line::raw(""),
        Line::from(" History:  e or Enter to edit any past session · u for unassigned"),
        Line::from(" CLI    :  tomatui PROJECT --25 --2   |   tomatui stats --json"),
        Line::raw(""),
        Line::from(Span::styled(
            " Press Esc, ?, or Enter to close",
            Style::default().fg(theme.primary),
        )),
    ];
    f.render_widget(Paragraph::new(lines), inner);
}

fn modal_block<'a>(title: &'a str, theme: &Theme) -> Block<'a> {
    Block::default()
        .title(Span::styled(
            title,
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.primary))
        .style(Style::default().bg(theme.bg).fg(theme.fg))
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let w = width.min(area.width.saturating_sub(2));
    let h = height.min(area.height.saturating_sub(2));
    Rect::new(
        area.x + (area.width.saturating_sub(w)) / 2,
        area.y + (area.height.saturating_sub(h)) / 2,
        w,
        h,
    )
}
