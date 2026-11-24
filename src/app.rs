use crate::audio;
use crate::cli::LaunchOptions;
use crate::config::Config;
use crate::db::{
    Activity, Database, Project, SessionFilter, SessionRecord, SessionStatus, StatsRange,
    StatsReport,
};
use crate::timer::{PomodoroPhase, PomodoroTimer, SessionPlan, TimerEvent, TimerStatus};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;
use std::io::{stdout, Write};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTab {
    Timer,
    Projects,
    History,
    Stats,
    Settings,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModalState {
    None,
    Help,
    /// Plan editor: presets or 3 manual numeric inputs (work, break=5, batches=1)
    PlanEditor {
        focus: usize, // 0=work, 1=break, 2=batches
        work_buf: String,
        break_buf: String,
        batches_buf: String,
    },
    /// Project & Activity picker (works in Opening View, mid-session, or History Edit)
    ContextPicker {
        /// None = editing active timer context; Some(session_id) = editing historical session
        session_id: Option<String>,
        focus: usize, // 0=project list/input, 1=activity list/input, 2=status (history only), 3=notes (history only)
        project_input: String,
        activity_input: String,
        status: SessionStatus,
        notes_input: String,
        project_idx: usize,
        activity_idx: usize,
    },
    /// Quick note editor for active/upcoming session
    NoteEditor {
        buf: String,
    },
    /// Add new project or activity from Projects tab
    AddItem {
        is_activity: bool,
        name_buf: String,
        est_buf: String,
        focus: usize,
    },
    /// Edit SQLite database file location from Settings
    DbPathEditor {
        buf: String,
    },
}

#[derive(Debug, Clone, Default)]
pub struct ClickTargets {
    pub tab_timer: Rect,
    pub tab_projects: Rect,
    pub tab_history: Rect,
    pub tab_stats: Rect,
    pub tab_settings: Rect,
    pub opening_project_row: Rect,
    pub opening_plan_row: Rect,
    pub opening_start_btn: Rect,
    pub active_pause_btn: Rect,
    pub active_stop_btn: Rect,
}

pub struct App {
    pub db: Database,
    pub config: Config,
    pub timer: PomodoroTimer,
    pub active_tab: ActiveTab,
    pub previous_tab: ActiveTab,
    pub modal: ModalState,
    pub should_quit: bool,
    pub exit_on_batch_done: bool,

    // Active timer context
    pub active_project: Option<Project>,
    pub active_activity: Option<Activity>,
    pub active_note: String,
    pub active_session_id: Option<String>,

    // Opening view row focus (0=Project, 1=Plan, 2=Start)
    pub opening_focus: usize,

    // Cached views refreshed from SQLite
    pub projects: Vec<Project>,
    pub project_activities: Vec<Activity>,
    pub selected_project_idx: usize,
    pub selected_activity_idx: usize,
    pub projects_pane_focus: usize, // 0=projects, 1=activities

    pub sessions: Vec<SessionRecord>,
    pub selected_session_idx: usize,
    pub history_unassigned_only: bool,

    pub stats_range: StatsRange,
    pub stats_report: StatsReport,

    pub settings_idx: usize,
    pub status_message: Option<String>,
    pub status_ticks: usize,
    pub tick_count: u32,
    pub click_targets: ClickTargets,
}

impl App {
    pub fn new(db: Database, launch: LaunchOptions) -> Self {
        let config = db.load_config();
        let work_mins = launch.work_mins.unwrap_or(config.work_duration_mins);
        let break_mins = launch.break_mins.unwrap_or(config.short_break_mins);
        let batches = launch.batches.unwrap_or(1);
        let plan = SessionPlan::new(work_mins, break_mins, batches);
        let timer = PomodoroTimer::with_plan(plan);

        let active_project = launch
            .project
            .as_deref()
            .and_then(|name| db.get_or_create_project(name, None).ok());
        let active_activity = match (&active_project, launch.activity.as_deref()) {
            (Some(proj), Some(act_name)) => db.get_or_create_activity(&proj.id, act_name, 1).ok(),
            _ => None,
        };

        let stats_report = db
            .get_stats(StatsRange::Week, None)
            .unwrap_or_else(|_| empty_stats(StatsRange::Week));

        let mut app = Self {
            db,
            config,
            timer,
            active_tab: ActiveTab::Timer,
            previous_tab: ActiveTab::Timer,
            modal: ModalState::None,
            should_quit: false,
            exit_on_batch_done: launch.exit_on_done,
            active_project,
            active_activity,
            active_note: String::new(),
            active_session_id: None,
            opening_focus: 2, // Highlight Start Focus by default for 1-keystroke start
            projects: Vec::new(),
            project_activities: Vec::new(),
            selected_project_idx: 0,
            selected_activity_idx: 0,
            projects_pane_focus: 0,
            sessions: Vec::new(),
            selected_session_idx: 0,
            history_unassigned_only: false,
            stats_range: StatsRange::Week,
            stats_report,
            settings_idx: 0,
            status_message: None,
            status_ticks: 0,
            tick_count: 0,
            click_targets: ClickTargets::default(),
        };

        app.refresh_data();

        if launch.autostart {
            app.start_focus_session();
        }
        app
    }

    pub fn refresh_data(&mut self) {
        self.projects = self.db.list_projects(false).unwrap_or_default();
        if self.selected_project_idx >= self.projects.len() {
            self.selected_project_idx = self.projects.len().saturating_sub(1);
        }
        let sel_proj_id = self
            .projects
            .get(self.selected_project_idx)
            .map(|p| p.id.clone());
        self.project_activities = match sel_proj_id.as_deref() {
            Some(pid) => self.db.list_activities(Some(pid), true).unwrap_or_default(),
            None => Vec::new(),
        };
        if self.selected_activity_idx >= self.project_activities.len() {
            self.selected_activity_idx = self.project_activities.len().saturating_sub(1);
        }

        self.sessions = self
            .db
            .list_sessions(&SessionFilter {
                unassigned_only: self.history_unassigned_only,
                limit: Some(100),
                ..Default::default()
            })
            .unwrap_or_default();
        if self.selected_session_idx >= self.sessions.len() {
            self.selected_session_idx = self.sessions.len().saturating_sub(1);
        }

        if let Ok(report) = self.db.get_stats(self.stats_range, None) {
            self.stats_report = report;
        }
    }

    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some(msg.into());
        self.status_ticks = 32; // ~8 seconds at 250ms ticks
    }

    pub fn active_context_label(&self) -> String {
        match (&self.active_project, &self.active_activity) {
            (Some(p), Some(a)) => format!("{} / {}", p.name, a.name),
            (Some(p), None) => p.name.clone(),
            (None, _) => "Unassigned (tag later)".to_string(),
        }
    }

    pub fn start_focus_session(&mut self) {
        self.timer.start();
        if self.timer.phase == PomodoroPhase::Work {
            if let Ok(rec) = self.db.start_session(
                PomodoroPhase::Work,
                self.timer.plan.work_mins,
                self.active_project.as_ref().map(|p| p.id.as_str()),
                self.active_activity.as_ref().map(|a| a.id.as_str()),
                &self.active_note,
            ) {
                self.active_session_id = Some(rec.id);
                self.refresh_data();
            }
        }
    }

    pub fn stop_active_session(&mut self) {
        let phase = self.timer.phase;
        let spent = self.timer.stop_to_idle();
        if let Some(sid) = self.active_session_id.take() {
            let status = if spent >= 60 {
                SessionStatus::Partial
            } else {
                SessionStatus::Skipped
            };
            let _ = self.db.finish_session(&sid, status, spent);
            if phase == PomodoroPhase::Work {
                self.set_status(format!(
                    "Stopped ({}m logged as {})",
                    (spent + 30) / 60,
                    status.as_str()
                ));
            }
            self.refresh_data();
        }
    }

    pub fn on_tick(&mut self) {
        self.tick_count = self.tick_count.wrapping_add(1);
        if self.tick_count % 4 == 0 {
            self.tick_second();
        }
        if self.status_ticks > 0 {
            self.status_ticks -= 1;
            if self.status_ticks == 0 {
                self.status_message = None;
            }
        }
    }

    fn tick_second(&mut self) {
        if let Some(TimerEvent::PhaseCompleted {
            finished_phase,
            next_phase,
            actual_secs,
            batch_finished,
        }) = self.timer.tick(&self.config)
        {
            if let Some(sid) = self.active_session_id.take() {
                let _ = self
                    .db
                    .finish_session(&sid, SessionStatus::Completed, actual_secs);
            } else {
                // Record break completion if desired
                if let Ok(rec) = self.db.start_session(
                    finished_phase,
                    (actual_secs / 60).max(1),
                    self.active_project.as_ref().map(|p| p.id.as_str()),
                    self.active_activity.as_ref().map(|a| a.id.as_str()),
                    "",
                ) {
                    let _ = self
                        .db
                        .finish_session(&rec.id, SessionStatus::Completed, actual_secs);
                }
            }

            self.notify_phase(finished_phase, next_phase, batch_finished);

            if batch_finished {
                self.set_status(format!(
                    "Batch complete! ({} × {}m)",
                    self.timer.plan.batches, self.timer.plan.work_mins
                ));
                self.refresh_data();
                if self.exit_on_batch_done {
                    self.should_quit = true;
                }
                return;
            }

            // If next phase auto-started as Work, open a fresh DB session row
            if next_phase == PomodoroPhase::Work && self.timer.status == TimerStatus::Running {
                if let Ok(rec) = self.db.start_session(
                    PomodoroPhase::Work,
                    self.timer.plan.work_mins,
                    self.active_project.as_ref().map(|p| p.id.as_str()),
                    self.active_activity.as_ref().map(|a| a.id.as_str()),
                    &self.active_note,
                ) {
                    self.active_session_id = Some(rec.id);
                }
            }

            self.set_status(format!(
                "{} finished · Next: {}",
                finished_phase.label(),
                next_phase.label()
            ));
            self.refresh_data();
        }
    }

    fn notify_phase(&self, finished: PomodoroPhase, next: PomodoroPhase, batch_finished: bool) {
        if self.config.sound_enabled {
            audio::play_phase_chime(finished);
            let mut out = stdout();
            let _ = out.write_all(b"\x07");
            let _ = out.flush();
        }
        if self.config.desktop_notifications {
            let summary = if batch_finished {
                "tomatui — Batch Complete!".to_string()
            } else {
                format!("tomatui — {} Finished", finished.label())
            };
            let body = if batch_finished {
                format!(
                    "Finished {} × {}m focus sessions.",
                    self.timer.plan.batches, self.timer.plan.work_mins
                )
            } else {
                format!("Next up: {}", next.label())
            };
            std::thread::spawn(move || {
                let _ = notify_rust::Notification::new()
                    .summary(&summary)
                    .body(&body)
                    .appname("tomatui")
                    .show();
            });
        }
    }

    pub fn open_plan_modal(&mut self) {
        self.modal = ModalState::PlanEditor {
            focus: 0,
            work_buf: self.timer.plan.work_mins.to_string(),
            break_buf: self.timer.plan.break_mins.to_string(),
            batches_buf: self.timer.plan.batches.to_string(),
        };
    }

    pub fn open_context_picker(&mut self, session_id: Option<String>) {
        self.refresh_data();
        let (proj_input, act_input, status, notes) = if let Some(ref sid) = session_id {
            if let Ok(Some(s)) = self.db.get_session_by_id(sid) {
                (
                    s.project_name.unwrap_or_default(),
                    s.activity_name.unwrap_or_default(),
                    s.status,
                    s.notes,
                )
            } else {
                (String::new(), String::new(), SessionStatus::Completed, String::new())
            }
        } else {
            (
                self.active_project
                    .as_ref()
                    .map(|p| p.name.clone())
                    .unwrap_or_default(),
                self.active_activity
                    .as_ref()
                    .map(|a| a.name.clone())
                    .unwrap_or_default(),
                SessionStatus::Completed,
                self.active_note.clone(),
            )
        };

        self.modal = ModalState::ContextPicker {
            session_id,
            focus: 0,
            project_input: proj_input,
            activity_input: act_input,
            status,
            notes_input: notes,
            project_idx: 0,
            activity_idx: 0,
        };
    }

    pub fn on_mouse(&mut self, mouse: MouseEvent) {
        if !matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
            return;
        }
        if !matches!(self.modal, ModalState::None) {
            return;
        }
        let col = mouse.column;
        let row = mouse.row;
        let hit = |r: Rect| col >= r.x && col < r.x + r.width && row >= r.y && row < r.y + r.height;

        if hit(self.click_targets.tab_timer) {
            self.active_tab = ActiveTab::Timer;
        } else if hit(self.click_targets.tab_projects) {
            self.active_tab = ActiveTab::Projects;
            self.refresh_data();
        } else if hit(self.click_targets.tab_history) {
            self.active_tab = ActiveTab::History;
            self.refresh_data();
        } else if hit(self.click_targets.tab_stats) {
            self.active_tab = ActiveTab::Stats;
            self.refresh_data();
        } else if hit(self.click_targets.tab_settings) {
            self.active_tab = ActiveTab::Settings;
        } else if self.active_tab == ActiveTab::Timer {
            if self.timer.status == TimerStatus::Idle {
                if hit(self.click_targets.opening_project_row) {
                    self.opening_focus = 0;
                    self.open_context_picker(None);
                } else if hit(self.click_targets.opening_plan_row) {
                    self.opening_focus = 1;
                    self.open_plan_modal();
                } else if hit(self.click_targets.opening_start_btn) {
                    self.opening_focus = 2;
                    self.start_focus_session();
                }
            } else if hit(self.click_targets.active_pause_btn) {
                self.timer.toggle_pause();
            } else if hit(self.click_targets.active_stop_btn) {
                self.stop_active_session();
            }
        }
    }

    pub fn on_key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.stop_active_session();
            self.should_quit = true;
            return;
        }

        if !matches!(self.modal, ModalState::None) {
            self.handle_modal_key(key);
            return;
        }

        // Global navigation keys (never conflict with sub-view filters!)
        match key.code {
            KeyCode::Char('q') => {
                if self.timer.status != TimerStatus::Idle {
                    self.stop_active_session();
                }
                self.should_quit = true;
                return;
            }
            KeyCode::Char('?') => {
                self.modal = ModalState::Help;
                return;
            }
            KeyCode::Char('1') | KeyCode::Char('T') => {
                self.switch_tab(ActiveTab::Timer);
                return;
            }
            KeyCode::Char('2') | KeyCode::Char('P') => {
                self.switch_tab(ActiveTab::Projects);
                self.refresh_data();
                return;
            }
            KeyCode::Char('3') | KeyCode::Char('H') => {
                self.switch_tab(ActiveTab::History);
                self.refresh_data();
                return;
            }
            KeyCode::Char('4') | KeyCode::Char('S') => {
                self.switch_tab(ActiveTab::Stats);
                self.refresh_data();
                return;
            }
            KeyCode::Char(',') => {
                self.switch_tab(ActiveTab::Settings);
                return;
            }
            KeyCode::Tab => {
                let next = match self.active_tab {
                    ActiveTab::Timer => ActiveTab::Projects,
                    ActiveTab::Projects => ActiveTab::History,
                    ActiveTab::History => ActiveTab::Stats,
                    ActiveTab::Stats => ActiveTab::Settings,
                    ActiveTab::Settings => ActiveTab::Timer,
                };
                self.switch_tab(next);
                self.refresh_data();
                return;
            }
            KeyCode::BackTab => {
                let prev = match self.active_tab {
                    ActiveTab::Timer => ActiveTab::Settings,
                    ActiveTab::Projects => ActiveTab::Timer,
                    ActiveTab::History => ActiveTab::Projects,
                    ActiveTab::Stats => ActiveTab::History,
                    ActiveTab::Settings => ActiveTab::Stats,
                };
                self.switch_tab(prev);
                self.refresh_data();
                return;
            }
            _ => {}
        }

        match self.active_tab {
            ActiveTab::Timer => self.handle_timer_key(key),
            ActiveTab::Projects => self.handle_projects_key(key),
            ActiveTab::History => self.handle_history_key(key),
            ActiveTab::Stats => self.handle_stats_key(key),
            ActiveTab::Settings => self.handle_settings_key(key),
        }
    }

    fn handle_timer_key(&mut self, key: KeyEvent) {
        if self.timer.status == TimerStatus::Idle {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    self.opening_focus = self.opening_focus.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.opening_focus = (self.opening_focus + 1).min(2);
                }
                KeyCode::Char('p') => self.open_context_picker(None),
                KeyCode::Char('t') => self.open_plan_modal(),
                KeyCode::Char('n') => {
                    self.modal = ModalState::NoteEditor {
                        buf: self.active_note.clone(),
                    };
                }
                KeyCode::Char(' ') => self.start_focus_session(),
                KeyCode::Enter => match self.opening_focus {
                    0 => self.open_context_picker(None),
                    1 => self.open_plan_modal(),
                    _ => self.start_focus_session(),
                },
                _ => {}
            }
        } else {
            match key.code {
                KeyCode::Char(' ') => {
                    self.timer.toggle_pause();
                    // If resuming into a Work phase that had no DB row yet
                    if self.timer.status == TimerStatus::Running
                        && self.timer.phase == PomodoroPhase::Work
                        && self.active_session_id.is_none()
                    {
                        if let Ok(rec) = self.db.start_session(
                            PomodoroPhase::Work,
                            self.timer.plan.work_mins,
                            self.active_project.as_ref().map(|p| p.id.as_str()),
                            self.active_activity.as_ref().map(|a| a.id.as_str()),
                            &self.active_note,
                        ) {
                            self.active_session_id = Some(rec.id);
                        }
                    }
                }
                KeyCode::Char('s') => {
                    let (_, spent, batch_done) = self.timer.skip(&self.config);
                    if let Some(sid) = self.active_session_id.take() {
                        let _ = self.db.finish_session(&sid, SessionStatus::Skipped, spent);
                    }
                    if !batch_done
                        && self.timer.phase == PomodoroPhase::Work
                        && self.timer.status == TimerStatus::Running
                    {
                        if let Ok(rec) = self.db.start_session(
                            PomodoroPhase::Work,
                            self.timer.plan.work_mins,
                            self.active_project.as_ref().map(|p| p.id.as_str()),
                            self.active_activity.as_ref().map(|a| a.id.as_str()),
                            &self.active_note,
                        ) {
                            self.active_session_id = Some(rec.id);
                        }
                    }
                    self.set_status("Skipped phase");
                    self.refresh_data();
                }
                KeyCode::Esc => self.stop_active_session(),
                KeyCode::Char('p') => self.open_context_picker(None),
                KeyCode::Char('n') => {
                    self.modal = ModalState::NoteEditor {
                        buf: self.active_note.clone(),
                    };
                }
                _ => {}
            }
        }
    }

    fn handle_projects_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Left | KeyCode::Char('h') => self.projects_pane_focus = 0,
            KeyCode::Right | KeyCode::Char('l') => self.projects_pane_focus = 1,
            KeyCode::Up | KeyCode::Char('k') => {
                if self.projects_pane_focus == 0 {
                    if !self.projects.is_empty() {
                        self.selected_project_idx = if self.selected_project_idx == 0 {
                            self.projects.len() - 1
                        } else {
                            self.selected_project_idx - 1
                        };
                        self.selected_activity_idx = 0;
                        self.refresh_data();
                    }
                } else if !self.project_activities.is_empty() {
                    self.selected_activity_idx = if self.selected_activity_idx == 0 {
                        self.project_activities.len() - 1
                    } else {
                        self.selected_activity_idx - 1
                    };
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.projects_pane_focus == 0 {
                    if !self.projects.is_empty() {
                        self.selected_project_idx =
                            (self.selected_project_idx + 1) % self.projects.len();
                        self.selected_activity_idx = 0;
                        self.refresh_data();
                    }
                } else if !self.project_activities.is_empty() {
                    self.selected_activity_idx =
                        (self.selected_activity_idx + 1) % self.project_activities.len();
                }
            }
            KeyCode::Char('a') => {
                let is_act = self.projects_pane_focus == 1 && !self.projects.is_empty();
                self.modal = ModalState::AddItem {
                    is_activity: is_act,
                    name_buf: String::new(),
                    est_buf: "2".to_string(),
                    focus: 0,
                };
            }
            KeyCode::Char(' ') => {
                if self.projects_pane_focus == 1 {
                    if let Some(act) = self.project_activities.get(self.selected_activity_idx) {
                        let id = act.id.clone();
                        let _ = self.db.toggle_activity_completed(&id);
                        self.refresh_data();
                    }
                }
            }
            KeyCode::Enter => {
                if let Some(proj) = self.projects.get(self.selected_project_idx).cloned() {
                    self.active_project = Some(proj);
                    self.active_activity = if self.projects_pane_focus == 1 {
                        self.project_activities
                            .get(self.selected_activity_idx)
                            .cloned()
                    } else {
                        None
                    };
                    self.active_tab = ActiveTab::Timer;
                    self.set_status(format!("Selected: {}", self.active_context_label()));
                }
            }
            KeyCode::Char('d') => {
                if self.projects_pane_focus == 1 {
                    if let Some(act) = self.project_activities.get(self.selected_activity_idx) {
                        let id = act.id.clone();
                        let _ = self.db.delete_activity(&id);
                        self.set_status("Deleted activity");
                        self.refresh_data();
                    }
                } else if let Some(proj) = self.projects.get(self.selected_project_idx) {
                    let id = proj.id.clone();
                    let name = proj.name.clone();
                    let _ = self.db.set_project_archived(&id, true);
                    self.set_status(format!("Archived project: {}", name));
                    self.refresh_data();
                }
            }
            _ => {}
        }
    }

    fn handle_history_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if !self.sessions.is_empty() {
                    self.selected_session_idx = if self.selected_session_idx == 0 {
                        self.sessions.len() - 1
                    } else {
                        self.selected_session_idx - 1
                    };
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !self.sessions.is_empty() {
                    self.selected_session_idx =
                        (self.selected_session_idx + 1) % self.sessions.len();
                }
            }
            KeyCode::Char('u') => {
                self.history_unassigned_only = !self.history_unassigned_only;
                self.selected_session_idx = 0;
                self.refresh_data();
            }
            KeyCode::Char('e') | KeyCode::Enter => {
                if let Some(sess) = self.sessions.get(self.selected_session_idx) {
                    let sid = sess.id.clone();
                    self.open_context_picker(Some(sid));
                }
            }
            KeyCode::Char('d') => {
                if let Some(sess) = self.sessions.get(self.selected_session_idx) {
                    let sid = sess.id.clone();
                    let _ = self.db.delete_session(&sid);
                    self.set_status("Deleted session");
                    self.refresh_data();
                }
            }
            _ => {}
        }
    }

    fn handle_stats_key(&mut self, key: KeyEvent) {
        if let KeyCode::Char('r') | KeyCode::Right | KeyCode::Char('l') = key.code {
            self.stats_range = self.stats_range.next();
            self.refresh_data();
        }
    }

    fn switch_tab(&mut self, next: ActiveTab) {
        if self.active_tab != next {
            self.previous_tab = self.active_tab;
            self.active_tab = next;
        }
    }

    fn handle_settings_key(&mut self, key: KeyEvent) {
        const TOTAL_ITEMS: usize = 10;
        match key.code {
            KeyCode::Esc => {
                let target = if self.previous_tab == ActiveTab::Settings {
                    ActiveTab::Timer
                } else {
                    self.previous_tab
                };
                self.active_tab = target;
                self.refresh_data();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.settings_idx = if self.settings_idx == 0 {
                    TOTAL_ITEMS - 1
                } else {
                    self.settings_idx - 1
                };
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.settings_idx = (self.settings_idx + 1) % TOTAL_ITEMS;
            }
            KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('-') => {
                self.adjust_setting(-1);
            }
            KeyCode::Right | KeyCode::Char('l') | KeyCode::Char('+') | KeyCode::Char('=') => {
                self.adjust_setting(1);
            }
            KeyCode::Char(' ') | KeyCode::Enter => {
                if self.settings_idx == 9 {
                    self.modal = ModalState::DbPathEditor {
                        buf: Database::display_short_path(self.db.path()),
                    };
                } else {
                    self.adjust_setting(1);
                }
            }
            _ => {}
        }
    }

    fn adjust_setting(&mut self, delta: i32) {
        match self.settings_idx {
            0 => {
                self.config.work_duration_mins =
                    (self.config.work_duration_mins as i32 + delta).clamp(1, 120) as u32;
                if self.timer.status == TimerStatus::Idle {
                    let plan = SessionPlan::new(
                        self.config.work_duration_mins,
                        self.timer.plan.break_mins,
                        self.timer.plan.batches,
                    );
                    self.timer.set_plan(plan);
                }
            }
            1 => {
                self.config.short_break_mins =
                    (self.config.short_break_mins as i32 + delta).clamp(1, 60) as u32;
            }
            2 => {
                self.config.long_break_mins =
                    (self.config.long_break_mins as i32 + delta).clamp(1, 90) as u32;
            }
            3 => {
                self.config.long_break_interval =
                    (self.config.long_break_interval as i32 + delta).clamp(1, 24) as u32;
            }
            4 => self.config.auto_start_breaks = !self.config.auto_start_breaks,
            5 => self.config.auto_start_work = !self.config.auto_start_work,
            6 => self.config.sound_enabled = !self.config.sound_enabled,
            7 => self.config.desktop_notifications = !self.config.desktop_notifications,
            8 => {
                self.config.theme = if delta >= 0 {
                    self.config.theme.next()
                } else {
                    self.config.theme.prev()
                };
            }
            _ => {}
        }
        let _ = self.db.save_config(&self.config);
    }

    fn handle_modal_key(&mut self, key: KeyEvent) {
        let mut modal = std::mem::replace(&mut self.modal, ModalState::None);
        match &mut modal {
            ModalState::None => {}
            ModalState::Help => {
                if matches!(
                    key.code,
                    KeyCode::Esc | KeyCode::Char('?') | KeyCode::Enter | KeyCode::Char('q')
                ) {
                    return;
                }
                self.modal = modal;
            }
            ModalState::PlanEditor {
                focus,
                work_buf,
                break_buf,
                batches_buf,
            } => match key.code {
                KeyCode::Esc => return,
                KeyCode::Tab | KeyCode::Right => {
                    *focus = (*focus + 1) % 3;
                    self.modal = modal;
                }
                KeyCode::BackTab | KeyCode::Left => {
                    *focus = (*focus + 2) % 3;
                    self.modal = modal;
                }
                KeyCode::Char('a') => {
                    // Quick preset 1: 25m x 1
                    self.timer.set_plan(SessionPlan::new(25, 5, 1));
                    self.set_status("Plan: 25m × 1");
                    return;
                }
                KeyCode::Char('b') => {
                    // Quick preset 2: 25m x 2
                    self.timer.set_plan(SessionPlan::new(25, 5, 2));
                    self.set_status("Plan: 25m × 2 (5m break)");
                    return;
                }
                KeyCode::Char('c') => {
                    // Quick preset 3: 50m x 2
                    self.timer.set_plan(SessionPlan::new(50, 10, 2));
                    self.set_status("Plan: 50m × 2 (10m break)");
                    return;
                }
                KeyCode::Backspace => {
                    let target = match *focus {
                        0 => work_buf,
                        1 => break_buf,
                        _ => batches_buf,
                    };
                    target.pop();
                    self.modal = modal;
                }
                KeyCode::Char(ch) if ch.is_ascii_digit() => {
                    let target = match *focus {
                        0 => work_buf,
                        1 => break_buf,
                        _ => batches_buf,
                    };
                    if target.len() < 3 {
                        target.push(ch);
                    }
                    self.modal = modal;
                }
                KeyCode::Enter => {
                    let w = work_buf
                        .parse::<u32>()
                        .unwrap_or(self.config.work_duration_mins);
                    let b = break_buf.parse::<u32>().unwrap_or(5);
                    let n = batches_buf.parse::<u32>().unwrap_or(1);
                    let plan = SessionPlan::new(w, b, n);
                    self.timer.set_plan(plan);
                    self.set_status(format!("Plan set: {}", plan.label()));
                    return;
                }
                _ => self.modal = modal,
            },
            ModalState::NoteEditor { buf } => match key.code {
                KeyCode::Esc => return,
                KeyCode::Backspace => {
                    buf.pop();
                    self.modal = modal;
                }
                KeyCode::Char(ch) => {
                    buf.push(ch);
                    self.modal = modal;
                }
                KeyCode::Enter => {
                    let note = buf.trim().to_string();
                    self.active_note = note.clone();
                    if let Some(ref sid) = self.active_session_id {
                        let _ = self.db.edit_session(sid, None, None, None, Some(&note));
                        self.refresh_data();
                    }
                    self.set_status("Note updated");
                    return;
                }
                _ => self.modal = modal,
            },
            ModalState::DbPathEditor { buf } => match key.code {
                KeyCode::Esc => return,
                KeyCode::Backspace => {
                    buf.pop();
                    self.modal = modal;
                }
                KeyCode::Char(ch) => {
                    buf.push(ch);
                    self.modal = modal;
                }
                KeyCode::Enter => {
                    match self.db.relocate(buf) {
                        Ok(new_db) => {
                            let display = Database::display_short_path(new_db.path());
                            self.db = new_db;
                            let _ = self.db.save_config(&self.config);
                            self.refresh_data();
                            self.set_status(format!("Data moved to {}", display));
                        }
                        Err(err) => {
                            self.set_status(format!("Could not change path: {}", err));
                        }
                    }
                    return;
                }
                _ => self.modal = modal,
            },
            ModalState::AddItem {
                is_activity,
                name_buf,
                est_buf,
                focus,
            } => match key.code {
                KeyCode::Esc => return,
                KeyCode::Tab | KeyCode::Down | KeyCode::Up if *is_activity => {
                    *focus = 1 - *focus;
                    self.modal = modal;
                }
                KeyCode::Backspace => {
                    if *focus == 0 {
                        name_buf.pop();
                    } else {
                        est_buf.pop();
                    }
                    self.modal = modal;
                }
                KeyCode::Char(ch) => {
                    if *focus == 0 {
                        name_buf.push(ch);
                    } else if ch.is_ascii_digit() && est_buf.len() < 2 {
                        est_buf.push(ch);
                    }
                    self.modal = modal;
                }
                KeyCode::Enter => {
                    let name = name_buf.trim().to_string();
                    if !name.is_empty() {
                        if *is_activity {
                            if let Some(proj) = self.projects.get(self.selected_project_idx) {
                                let est = est_buf.parse::<u32>().unwrap_or(1).max(1);
                                let _ = self.db.get_or_create_activity(&proj.id, &name, est);
                                self.set_status(format!("Added activity: {}", name));
                            }
                        } else {
                            let _ = self.db.get_or_create_project(&name, None);
                            self.set_status(format!("Added project: {}", name));
                        }
                        self.refresh_data();
                    }
                    return;
                }
                _ => self.modal = modal,
            },
            ModalState::ContextPicker {
                session_id,
                focus,
                project_input,
                activity_input,
                status,
                notes_input,
                project_idx,
                activity_idx,
            } => {
                let is_history = session_id.is_some();
                let max_focus = if is_history { 4 } else { 2 };
                match key.code {
                    KeyCode::Esc => return,
                    KeyCode::Tab => {
                        *focus = (*focus + 1) % max_focus;
                        self.modal = modal;
                    }
                    KeyCode::BackTab => {
                        *focus = (*focus + max_focus - 1) % max_focus;
                        self.modal = modal;
                    }
                    KeyCode::Down if *focus == 0 && !self.projects.is_empty() => {
                        *project_idx = (*project_idx + 1) % (self.projects.len() + 1);
                        if *project_idx == 0 {
                            project_input.clear();
                        } else if let Some(p) = self.projects.get(*project_idx - 1) {
                            *project_input = p.name.clone();
                        }
                        self.modal = modal;
                    }
                    KeyCode::Up if *focus == 0 && !self.projects.is_empty() => {
                        let len = self.projects.len() + 1;
                        *project_idx = (*project_idx + len - 1) % len;
                        if *project_idx == 0 {
                            project_input.clear();
                        } else if let Some(p) = self.projects.get(*project_idx - 1) {
                            *project_input = p.name.clone();
                        }
                        self.modal = modal;
                    }
                    KeyCode::Down if *focus == 1 => {
                        if let Ok(Some(proj)) = self.db.find_project_by_name_or_id(project_input) {
                            if let Ok(acts) = self.db.list_activities(Some(&proj.id), false) {
                                if !acts.is_empty() {
                                    *activity_idx = (*activity_idx + 1) % (acts.len() + 1);
                                    if *activity_idx == 0 {
                                        activity_input.clear();
                                    } else if let Some(a) = acts.get(*activity_idx - 1) {
                                        *activity_input = a.name.clone();
                                    }
                                }
                            }
                        }
                        self.modal = modal;
                    }
                    KeyCode::Left | KeyCode::Right | KeyCode::Char(' ') if *focus == 2 => {
                        *status = status.next_editable();
                        self.modal = modal;
                    }
                    KeyCode::Backspace => {
                        match *focus {
                            0 => {
                                project_input.pop();
                            }
                            1 => {
                                activity_input.pop();
                            }
                            3 => {
                                notes_input.pop();
                            }
                            _ => {}
                        }
                        self.modal = modal;
                    }
                    KeyCode::Char(ch) => {
                        match *focus {
                            0 => project_input.push(ch),
                            1 => activity_input.push(ch),
                            3 => notes_input.push(ch),
                            _ => {}
                        }
                        self.modal = modal;
                    }
                    KeyCode::Enter => {
                        let p_trim = project_input.trim().to_string();
                        let a_trim = activity_input.trim().to_string();
                        let resolved_proj = if p_trim.is_empty() {
                            None
                        } else {
                            self.db.get_or_create_project(&p_trim, None).ok()
                        };
                        let resolved_act = match (&resolved_proj, a_trim.is_empty()) {
                            (Some(p), false) => {
                                self.db.get_or_create_activity(&p.id, &a_trim, 1).ok()
                            }
                            _ => None,
                        };

                        if let Some(sid) = session_id.clone() {
                            let _ = self.db.edit_session(
                                &sid,
                                Some(resolved_proj.as_ref().map(|p| p.id.as_str())),
                                Some(resolved_act.as_ref().map(|a| a.id.as_str())),
                                Some(*status),
                                Some(notes_input.trim()),
                            );
                            self.set_status("Session updated");
                        } else {
                            self.active_project = resolved_proj.clone();
                            self.active_activity = resolved_act.clone();
                            if let Some(ref sid) = self.active_session_id {
                                let _ = self.db.edit_session(
                                    sid,
                                    Some(resolved_proj.as_ref().map(|p| p.id.as_str())),
                                    Some(resolved_act.as_ref().map(|a| a.id.as_str())),
                                    None,
                                    None,
                                );
                            }
                            self.set_status(format!("Context: {}", self.active_context_label()));
                        }
                        self.refresh_data();
                        return;
                    }
                    _ => self.modal = modal,
                }
            }
        }
    }
}

fn empty_stats(range: StatsRange) -> StatsReport {
    StatsReport {
        range,
        today_sessions: 0,
        today_focus_mins: 0,
        range_sessions: 0,
        range_focus_mins: 0,
        current_streak_days: 0,
        best_streak_days: 0,
        unassigned_sessions: 0,
        last_7_days: Vec::new(),
        by_project: Vec::new(),
        by_activity: Vec::new(),
    }
}
