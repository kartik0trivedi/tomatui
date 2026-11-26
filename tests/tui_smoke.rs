use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::TestBackend, Terminal};
use tomatui::{
    app::{ActiveTab, App, ModalState},
    cli::LaunchOptions,
    db::Database,
    timer::TimerStatus,
    ui,
};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

#[test]
fn test_all_views_and_modals_render_without_panic() {
    tomatui::audio::set_audio_muted(true);
    let db = Database::open_in_memory().unwrap();
    let mut app = App::new(
        db,
        LaunchOptions {
            space: Some("Work".to_string()),
            project: Some("backend-api".to_string()),
            activity: Some("API schema".to_string()),
            work_mins: Some(25),
            break_mins: Some(5),
            batches: Some(2),
            autostart: false,
            exit_on_done: false,
        },
    );

    let backend = TestBackend::new(100, 30);
    let mut terminal = Terminal::new(backend).unwrap();

    // 1. Opening View
    assert_eq!(app.timer.status, TimerStatus::Idle);
    terminal.draw(|f| ui::render(f, &mut app)).unwrap();

    // 2. Plan modal (manual 3-number entry: 40m focus, 10m break, 3 batches)
    app.on_key(key(KeyCode::Char('t')));
    assert!(matches!(app.modal, ModalState::PlanEditor { .. }));
    terminal.draw(|f| ui::render(f, &mut app)).unwrap();
    // Clear "25" and type "40"
    app.on_key(key(KeyCode::Backspace));
    app.on_key(key(KeyCode::Backspace));
    app.on_key(key(KeyCode::Char('4')));
    app.on_key(key(KeyCode::Char('0')));
    // Tab to break, clear "5", type "10"
    app.on_key(key(KeyCode::Tab));
    app.on_key(key(KeyCode::Backspace));
    app.on_key(key(KeyCode::Char('1')));
    app.on_key(key(KeyCode::Char('0')));
    // Tab to batches, clear "2", type "3"
    app.on_key(key(KeyCode::Tab));
    app.on_key(key(KeyCode::Backspace));
    app.on_key(key(KeyCode::Char('3')));
    app.on_key(key(KeyCode::Enter));
    assert_eq!(app.timer.plan.work_mins, 40);
    assert_eq!(app.timer.plan.break_mins, 10);
    assert_eq!(app.timer.plan.batches, 3);

    // 3. Start Active Countdown
    app.on_key(key(KeyCode::Char(' ')));
    assert_eq!(app.timer.status, TimerStatus::Running);
    terminal.draw(|f| ui::render(f, &mut app)).unwrap();

    // 4. Switch through all tabs (Projects, History, Stats, Settings)
    for (ch, tab) in [
        ('2', ActiveTab::Projects),
        ('3', ActiveTab::History),
        ('4', ActiveTab::Stats),
        (',', ActiveTab::Settings),
    ] {
        app.on_key(key(KeyCode::Char(ch)));
        assert_eq!(app.active_tab, tab);
        terminal.draw(|f| ui::render(f, &mut app)).unwrap();
    }

    // 4b. Navigate out of Settings using Esc (back to Stats) and uppercase T/P/H/S
    app.on_key(key(KeyCode::Esc));
    assert_eq!(app.active_tab, ActiveTab::Stats);
    app.on_key(key(KeyCode::Char(',')));
    assert_eq!(app.active_tab, ActiveTab::Settings);
    app.on_key(key(KeyCode::Char('T')));
    assert_eq!(app.active_tab, ActiveTab::Timer);
    app.on_key(key(KeyCode::Char(',')));
    // Open Data Location editor modal on Settings row 10
    app.settings_idx = 10;
    app.on_key(key(KeyCode::Enter));
    assert!(matches!(app.modal, ModalState::DbPathEditor { .. }));
    terminal.draw(|f| ui::render(f, &mut app)).unwrap();
    app.on_key(key(KeyCode::Esc));

    // 5. Open Edit Session modal in History
    app.on_key(key(KeyCode::Char('3')));
    app.on_key(key(KeyCode::Char('e')));
    assert!(matches!(app.modal, ModalState::ContextPicker { .. }));
    terminal.draw(|f| ui::render(f, &mut app)).unwrap();
    app.on_key(key(KeyCode::Esc));

    // 6. Help modal
    app.on_key(key(KeyCode::Char('?')));
    assert!(matches!(app.modal, ModalState::Help));
    terminal.draw(|f| ui::render(f, &mut app)).unwrap();
    app.on_key(key(KeyCode::Esc));

    // 7. ContextPicker (`p`) shows and updates Space, Project, Activity
    app.on_key(key(KeyCode::Char('1')));
    app.on_key(key(KeyCode::Char('p')));
    assert!(matches!(app.modal, ModalState::ContextPicker { .. }));
    terminal.draw(|f| ui::render(f, &mut app)).unwrap();
    app.on_key(key(KeyCode::Esc));

    // 8. Projects tab (3-column: Spaces | Projects | Activities)
    // Pane 0 (Spaces): press 'a' to add a new Space "School", 'e' to edit Space
    app.on_key(key(KeyCode::Char('2')));
    assert_eq!(app.projects_pane_focus, 0);
    app.on_key(key(KeyCode::Char('a')));
    assert!(matches!(
        app.modal,
        ModalState::SpaceEditor {
            original_name: None,
            ..
        }
    ));
    terminal.draw(|f| ui::render(f, &mut app)).unwrap();
    for ch in "School".chars() {
        app.on_key(key(KeyCode::Char(ch)));
    }
    app.on_key(key(KeyCode::Enter));
    assert_eq!(app.spaces, vec!["School", "Work"]);
    assert_eq!(app.selected_projects_space(), Some("School"));
    terminal.draw(|f| ui::render(f, &mut app)).unwrap();

    // Edit Space via 'e'
    app.on_key(key(KeyCode::Char('e')));
    assert!(matches!(
        app.modal,
        ModalState::SpaceEditor {
            original_name: Some(_),
            ..
        }
    ));
    terminal.draw(|f| ui::render(f, &mut app)).unwrap();
    app.on_key(key(KeyCode::Esc));

    // Move right to Pane 1 (Projects): press 'a' — space is pre-filled with "School"
    app.on_key(key(KeyCode::Right));
    assert_eq!(app.projects_pane_focus, 1);
    app.on_key(key(KeyCode::Char('a')));
    assert!(matches!(
        app.modal,
        ModalState::ItemEditor {
            edit_id: None,
            is_activity: false,
            ..
        }
    ));
    for ch in "research-paper".chars() {
        app.on_key(key(KeyCode::Char(ch)));
    }
    app.on_key(key(KeyCode::Enter));
    assert_eq!(app.filtered_projects.len(), 1);
    assert_eq!(app.filtered_projects[0].name, "research-paper");
    assert_eq!(app.filtered_projects[0].space.as_deref(), Some("School"));
    terminal.draw(|f| ui::render(f, &mut app)).unwrap();

    // Edit Project via 'e'
    app.on_key(key(KeyCode::Char('e')));
    assert!(matches!(
        app.modal,
        ModalState::ItemEditor {
            edit_id: Some(_),
            is_activity: false,
            ..
        }
    ));
    terminal.draw(|f| ui::render(f, &mut app)).unwrap();
    app.on_key(key(KeyCode::Esc));

    // 9. Stats tab: left/right arrows cycle [All, School, Work]
    app.on_key(key(KeyCode::Char('4')));
    assert_eq!(app.active_stats_space(), None);
    app.on_key(key(KeyCode::Right));
    assert_eq!(app.active_stats_space(), Some("School"));
    terminal.draw(|f| ui::render(f, &mut app)).unwrap();
    app.on_key(key(KeyCode::Right));
    assert_eq!(app.active_stats_space(), Some("Work"));
    terminal.draw(|f| ui::render(f, &mut app)).unwrap();
    app.on_key(key(KeyCode::Left));
    assert_eq!(app.active_stats_space(), Some("School"));

    // 10. History tab: left/right arrows filter by Space and 'd' requires double-press confirm
    app.on_key(key(KeyCode::Char('3')));
    assert_eq!(app.active_history_space(), None);
    app.on_key(key(KeyCode::Right));
    assert_eq!(app.active_history_space(), Some("School"));
    terminal.draw(|f| ui::render(f, &mut app)).unwrap();
    app.on_key(key(KeyCode::Left));
    assert_eq!(app.active_history_space(), None);
    let initial_sessions = app.sessions.len();
    assert!(initial_sessions >= 1);
    app.on_key(key(KeyCode::Char('d')));
    assert!(app.pending_delete.is_some());
    assert_eq!(app.sessions.len(), initial_sessions); // Not deleted on first 'd'
    app.on_key(key(KeyCode::Char('d')));
    assert!(app.pending_delete.is_none());
    assert_eq!(app.sessions.len(), initial_sessions - 1); // Deleted on second 'd'

    // 11. Settings row 10 responds to 'e' (universal Edit)
    app.on_key(key(KeyCode::Char(',')));
    app.settings_idx = 10;
    app.on_key(key(KeyCode::Char('e')));
    assert!(matches!(app.modal, ModalState::DbPathEditor { .. }));
    app.on_key(key(KeyCode::Esc));
}
