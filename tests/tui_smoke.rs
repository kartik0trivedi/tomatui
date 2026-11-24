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
    // Open Data Location editor modal on Settings row 9
    app.settings_idx = 9;
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
}
