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
    /// Space, Project & Activity picker (works in Opening View, mid-session, or History Edit)
    ContextPicker {
        /// None = editing active timer context; Some(session_id) = editing historical session
        session_id: Option<String>,
        focus: usize, // 0=space, 1=project, 2=activity, 3=status (history only), 4=notes (history only)
        space_input: String,
        project_input: String,
        activity_input: String,
        status: SessionStatus,
        notes_input: String,
        space_idx: usize,
        project_idx: usize,
        activity_idx: usize,
    },
    /// Quick note editor for active/upcoming session
    NoteEditor {
        buf: String,
    },
    /// Add or rename a space from Spaces pane in Projects tab
    SpaceEditor {
        original_name: Option<String>,
        name_buf: String,
    },
    /// Add or edit a project (name + optional space) or activity (name + est) from Projects tab
    ItemEditor {
        edit_id: Option<String>,
        is_activity: bool,
        name_buf: String,
        space_buf: String,
        est_buf: String,
        focus: usize,
    },
    /// Edit SQLite database file location from Settings
    DbPathEditor {
        buf: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PendingDelete {
    Space(String),
    Project(String),
    Activity(String),
    Session(String),
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
    pub stats_space_pills: Vec<(Rect, usize)>,
    pub history_space_pills: Vec<(Rect, usize)>,
    pub history_unassigned_btn: Rect,
    pub history_list_area: Rect,
    pub projects_space_area: Rect,
    pub projects_list_area: Rect,
    pub projects_act_area: Rect,
    pub settings_list_area: Rect,
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
    pub spaces: Vec<String>,
    pub projects: Vec<Project>,
    pub filtered_projects: Vec<Project>,
    pub project_activities: Vec<Activity>,
    pub selected_space_idx: usize, // 0 = All, 1..=spaces.len() = specific space
    pub selected_project_idx: usize,
    pub selected_activity_idx: usize,
    pub projects_pane_focus: usize, // 0=spaces, 1=projects, 2=activities

    pub sessions: Vec<SessionRecord>,
    pub selected_session_idx: usize,
    pub history_space_idx: usize, // 0 = All, 1..=spaces.len() = specific space
    pub history_unassigned_only: bool,

    pub stats_range: StatsRange,
    pub stats_space_idx: usize, // 0 = All, 1..=spaces.len() = specific space
    pub stats_report: StatsReport,
    pub stats_view_report: StatsReport,

    pub settings_idx: usize,
    pub pending_delete: Option<PendingDelete>,
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

        let active_project = launch.project.as_deref().and_then(|name| {
            db.get_or_create_project_in_space(name, launch.space.as_deref(), None)
                .ok()
        });
        let active_activity = match (&active_project, launch.activity.as_deref()) {
            (Some(proj), Some(act_name)) => db.get_or_create_activity(&proj.id, act_name, 1).ok(),
            _ => None,
        };

        let stats_report = db
            .get_stats(StatsRange::Week, None, None)
            .unwrap_or_else(|_| empty_stats(StatsRange::Week));
        let stats_view_report = stats_report.clone();

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
            spaces: Vec::new(),
            projects: Vec::new(),
            filtered_projects: Vec::new(),
            project_activities: Vec::new(),
            selected_space_idx: 0,
            selected_project_idx: 0,
            selected_activity_idx: 0,
            projects_pane_focus: 0,
            sessions: Vec::new(),
            selected_session_idx: 0,
            history_space_idx: 0,
            history_unassigned_only: false,
            stats_range: StatsRange::Week,
            stats_space_idx: 0,
            stats_report,
            stats_view_report,
            settings_idx: 0,
            pending_delete: None,
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

    pub fn selected_projects_space(&self) -> Option<&str> {
        if self.selected_space_idx == 0 {
            None
        } else {
            self.spaces
                .get(self.selected_space_idx - 1)
                .map(String::as_str)
        }
    }

    pub fn active_history_space(&self) -> Option<&str> {
        if self.history_space_idx == 0 {
            None
        } else {
            self.spaces
                .get(self.history_space_idx - 1)
                .map(String::as_str)
        }
    }

    pub fn active_stats_space(&self) -> Option<&str> {
        if self.stats_space_idx == 0 {
            None
        } else {
            self.spaces
                .get(self.stats_space_idx - 1)
                .map(String::as_str)
        }
    }

    pub fn refresh_data(&mut self) {
        self.spaces = self.db.list_spaces().unwrap_or_default();
        if self.selected_space_idx > self.spaces.len() {
            self.selected_space_idx = 0;
        }
        if self.history_space_idx > self.spaces.len() {
            self.history_space_idx = 0;
        }
        if self.stats_space_idx > self.spaces.len() {
            self.stats_space_idx = 0;
        }
        self.projects = self.db.list_projects(false).unwrap_or_default();
        self.filtered_projects = match self.selected_projects_space() {
            None => self.projects.clone(),
            Some(sp) => self
                .projects
                .iter()
                .filter(|p| {
                    p.space
                        .as_deref()
                        .map(|s| s.eq_ignore_ascii_case(sp))
                        .unwrap_or(false)
                })
                .cloned()
                .collect(),
        };
        if self.selected_project_idx >= self.filtered_projects.len() {
            self.selected_project_idx = self.filtered_projects.len().saturating_sub(1);
        }
        let sel_proj_id = self
            .filtered_projects
            .get(self.selected_project_idx)
            .map(|p| p.id.clone());
        self.project_activities = match sel_proj_id.as_deref() {
            Some(pid) => self.db.list_activities(Some(pid), true).unwrap_or_default(),
            None => Vec::new(),
        };
        if self.selected_activity_idx >= self.project_activities.len() {
            self.selected_activity_idx = self.project_activities.len().saturating_sub(1);
        }

        let hist_space = if self.history_unassigned_only {
            None
        } else {
            self.active_history_space().map(ToString::to_string)
        };
        self.sessions = self
            .db
            .list_sessions(&SessionFilter {
                space: hist_space,
                unassigned_only: self.history_unassigned_only,
                limit: Some(100),
                ..Default::default()
            })
            .unwrap_or_default();
        if self.selected_session_idx >= self.sessions.len() {
            self.selected_session_idx = self.sessions.len().saturating_sub(1);
        }

        if let Ok(report) = self.db.get_stats(self.stats_range, None, None) {
            self.stats_report = report;
        }
        let space_filter = self.active_stats_space().map(ToString::to_string);
        if let Ok(filtered) = self
            .db
            .get_stats(self.stats_range, None, space_filter.as_deref())
        {
            self.stats_view_report = filtered;
        }
    }

    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some(msg.into());
        self.status_ticks = 32; // ~8 seconds at 250ms ticks
    }

    pub fn active_context_label(&self) -> String {
        match (&self.active_project, &self.active_activity) {
            (Some(p), Some(a)) => match p.space.as_deref().filter(|s| !s.is_empty()) {
                Some(sp) => format!("{} · {} / {}", sp, p.name, a.name),
                None => format!("{} / {}", p.name, a.name),
            },
            (Some(p), None) => match p.space.as_deref().filter(|s| !s.is_empty()) {
                Some(sp) => format!("{} · {}", sp, p.name),
                None => p.name.clone(),
            },
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
        if self.tick_count.is_multiple_of(4) {
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
        let (space_input, proj_input, act_input, status, notes) = if let Some(ref sid) = session_id
        {
            if let Ok(Some(s)) = self.db.get_session_by_id(sid) {
                (
                    s.space.unwrap_or_default(),
                    s.project_name.unwrap_or_default(),
                    s.activity_name.unwrap_or_default(),
                    s.status,
                    s.notes,
                )
            } else {
                (
                    String::new(),
                    String::new(),
                    String::new(),
                    SessionStatus::Completed,
                    String::new(),
                )
            }
        } else {
            (
                self.active_project
                    .as_ref()
                    .and_then(|p| p.space.clone())
                    .unwrap_or_default(),
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
            space_input,
            project_input: proj_input,
            activity_input: act_input,
            status,
            notes_input: notes,
            space_idx: 0,
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
            self.switch_tab(ActiveTab::Timer);
        } else if hit(self.click_targets.tab_projects) {
            self.switch_tab(ActiveTab::Projects);
            self.refresh_data();
        } else if hit(self.click_targets.tab_history) {
            self.switch_tab(ActiveTab::History);
            self.refresh_data();
        } else if hit(self.click_targets.tab_stats) {
            self.switch_tab(ActiveTab::Stats);
            self.refresh_data();
        } else if hit(self.click_targets.tab_settings) {
            self.switch_tab(ActiveTab::Settings);
        } else {
            self.pending_delete = None;
            match self.active_tab {
                ActiveTab::Timer => {
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
                ActiveTab::Projects => {
                    if hit(self.click_targets.projects_space_area) {
                        self.projects_pane_focus = 0;
                        let idx = (row - self.click_targets.projects_space_area.y) as usize;
                        if idx <= self.spaces.len() {
                            self.selected_space_idx = idx;
                            self.selected_project_idx = 0;
                            self.selected_activity_idx = 0;
                            self.refresh_data();
                        }
                    } else if hit(self.click_targets.projects_list_area) {
                        self.projects_pane_focus = 1;
                        let idx = (row - self.click_targets.projects_list_area.y) as usize;
                        if idx < self.filtered_projects.len() {
                            self.selected_project_idx = idx;
                            self.selected_activity_idx = 0;
                            self.refresh_data();
                        }
                    } else if hit(self.click_targets.projects_act_area) {
                        self.projects_pane_focus = 2;
                        let idx = (row - self.click_targets.projects_act_area.y) as usize;
                        if idx < self.project_activities.len() {
                            self.selected_activity_idx = idx;
                        }
                    }
                }
                ActiveTab::History => {
                    if hit(self.click_targets.history_unassigned_btn) {
                        self.history_unassigned_only = !self.history_unassigned_only;
                        if self.history_unassigned_only {
                            self.history_space_idx = 0;
                        }
                        self.selected_session_idx = 0;
                        self.refresh_data();
                    } else if let Some(&(_, idx)) = self
                        .click_targets
                        .history_space_pills
                        .iter()
                        .find(|(r, _)| hit(*r))
                    {
                        self.history_unassigned_only = false;
                        self.history_space_idx = idx;
                        self.selected_session_idx = 0;
                        self.refresh_data();
                    } else if hit(self.click_targets.history_list_area) {
                        let idx = (row - self.click_targets.history_list_area.y) as usize;
                        if idx < self.sessions.len() {
                            self.selected_session_idx = idx;
                        }
                    }
                }
                ActiveTab::Stats => {
                    if let Some(&(_, idx)) = self
                        .click_targets
                        .stats_space_pills
                        .iter()
                        .find(|(r, _)| hit(*r))
                    {
                        self.stats_space_idx = idx;
                        self.refresh_data();
                    }
                }
                ActiveTab::Settings => {
                    if hit(self.click_targets.settings_list_area) {
                        let idx = (row - self.click_targets.settings_list_area.y) as usize;
                        if idx < 11 {
                            self.settings_idx = idx;
                        }
                    }
                }
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
            self.pending_delete = None;
            self.handle_modal_key(key);
            return;
        }

        // Any key other than 'd' cancels a pending delete confirmation
        if key.code != KeyCode::Char('d') && self.pending_delete.is_some() {
            self.pending_delete = None;
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
            KeyCode::Left | KeyCode::Char('h') => {
                self.projects_pane_focus = self.projects_pane_focus.saturating_sub(1);
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.projects_pane_focus = (self.projects_pane_focus + 1).min(2);
            }
            KeyCode::Up | KeyCode::Char('k') => match self.projects_pane_focus {
                0 => {
                    let total = self.spaces.len() + 1;
                    self.selected_space_idx = (self.selected_space_idx + total - 1) % total;
                    self.selected_project_idx = 0;
                    self.selected_activity_idx = 0;
                    self.refresh_data();
                }
                1 => {
                    if !self.filtered_projects.is_empty() {
                        self.selected_project_idx = if self.selected_project_idx == 0 {
                            self.filtered_projects.len() - 1
                        } else {
                            self.selected_project_idx - 1
                        };
                        self.selected_activity_idx = 0;
                        self.refresh_data();
                    }
                }
                _ => {
                    if !self.project_activities.is_empty() {
                        self.selected_activity_idx = if self.selected_activity_idx == 0 {
                            self.project_activities.len() - 1
                        } else {
                            self.selected_activity_idx - 1
                        };
                    }
                }
            },
            KeyCode::Down | KeyCode::Char('j') => match self.projects_pane_focus {
                0 => {
                    let total = self.spaces.len() + 1;
                    self.selected_space_idx = (self.selected_space_idx + 1) % total;
                    self.selected_project_idx = 0;
                    self.selected_activity_idx = 0;
                    self.refresh_data();
                }
                1 => {
                    if !self.filtered_projects.is_empty() {
                        self.selected_project_idx =
                            (self.selected_project_idx + 1) % self.filtered_projects.len();
                        self.selected_activity_idx = 0;
                        self.refresh_data();
                    }
                }
                _ => {
                    if !self.project_activities.is_empty() {
                        self.selected_activity_idx =
                            (self.selected_activity_idx + 1) % self.project_activities.len();
                    }
                }
            },
            KeyCode::Char('a') => match self.projects_pane_focus {
                0 => {
                    self.modal = ModalState::SpaceEditor {
                        original_name: None,
                        name_buf: String::new(),
                    };
                }
                1 => {
                    let default_space = self
                        .selected_projects_space()
                        .unwrap_or_default()
                        .to_string();
                    self.modal = ModalState::ItemEditor {
                        edit_id: None,
                        is_activity: false,
                        name_buf: String::new(),
                        space_buf: default_space,
                        est_buf: "2".to_string(),
                        focus: 0,
                    };
                }
                _ => {
                    if !self.filtered_projects.is_empty() {
                        self.modal = ModalState::ItemEditor {
                            edit_id: None,
                            is_activity: true,
                            name_buf: String::new(),
                            space_buf: String::new(),
                            est_buf: "2".to_string(),
                            focus: 0,
                        };
                    }
                }
            },
            KeyCode::Char('e') => match self.projects_pane_focus {
                0 => {
                    if let Some(sp) = self.selected_projects_space().map(ToString::to_string) {
                        self.modal = ModalState::SpaceEditor {
                            original_name: Some(sp.clone()),
                            name_buf: sp,
                        };
                    }
                }
                1 => {
                    if let Some(proj) = self.filtered_projects.get(self.selected_project_idx) {
                        self.modal = ModalState::ItemEditor {
                            edit_id: Some(proj.id.clone()),
                            is_activity: false,
                            name_buf: proj.name.clone(),
                            space_buf: proj.space.clone().unwrap_or_default(),
                            est_buf: "2".to_string(),
                            focus: 0,
                        };
                    }
                }
                _ => {
                    if let Some(act) = self.project_activities.get(self.selected_activity_idx) {
                        self.modal = ModalState::ItemEditor {
                            edit_id: Some(act.id.clone()),
                            is_activity: true,
                            name_buf: act.name.clone(),
                            space_buf: String::new(),
                            est_buf: act.estimated_pomodoros.to_string(),
                            focus: 0,
                        };
                    }
                }
            },
            KeyCode::Char(' ') if self.projects_pane_focus == 2 => {
                if let Some(act) = self.project_activities.get(self.selected_activity_idx) {
                    let id = act.id.clone();
                    let _ = self.db.toggle_activity_completed(&id);
                    self.refresh_data();
                }
            }
            KeyCode::Enter => {
                if self.projects_pane_focus == 0 {
                    self.projects_pane_focus = 1;
                } else if let Some(proj) = self
                    .filtered_projects
                    .get(self.selected_project_idx)
                    .cloned()
                {
                    self.active_project = Some(proj);
                    self.active_activity = if self.projects_pane_focus == 2 {
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
            KeyCode::Char('d') => match self.projects_pane_focus {
                0 => {
                    if let Some(sp) = self.selected_projects_space().map(ToString::to_string) {
                        let target = PendingDelete::Space(sp.clone());
                        if self.pending_delete.as_ref() == Some(&target) {
                            self.pending_delete = None;
                            let _ = self.db.delete_space(&sp);
                            self.selected_space_idx = 0;
                            self.set_status(format!("Removed space: {}", sp));
                            self.refresh_data();
                        } else {
                            self.pending_delete = Some(target);
                            self.set_status(format!(
                                "Press 'd' again to remove space '{}' (projects kept)",
                                sp
                            ));
                        }
                    }
                }
                1 => {
                    if let Some(proj) = self.filtered_projects.get(self.selected_project_idx) {
                        let id = proj.id.clone();
                        let name = proj.name.clone();
                        let target = PendingDelete::Project(id.clone());
                        if self.pending_delete.as_ref() == Some(&target) {
                            self.pending_delete = None;
                            let _ = self.db.set_project_archived(&id, true);
                            self.set_status(format!("Archived project: {}", name));
                            self.refresh_data();
                        } else {
                            self.pending_delete = Some(target);
                            self.set_status(format!(
                                "Press 'd' again to archive project '{}'",
                                name
                            ));
                        }
                    }
                }
                _ => {
                    if let Some(act) = self.project_activities.get(self.selected_activity_idx) {
                        let id = act.id.clone();
                        let name = act.name.clone();
                        let target = PendingDelete::Activity(id.clone());
                        if self.pending_delete.as_ref() == Some(&target) {
                            self.pending_delete = None;
                            let _ = self.db.delete_activity(&id);
                            self.set_status(format!("Deleted activity: {}", name));
                            self.refresh_data();
                        } else {
                            self.pending_delete = Some(target);
                            self.set_status(format!(
                                "Press 'd' again to delete activity '{}'",
                                name
                            ));
                        }
                    }
                }
            },
            _ => {}
        }
    }

    fn handle_history_key(&mut self, key: KeyEvent) {
        let total_filters = self.spaces.len() + 1;
        match key.code {
            KeyCode::Right | KeyCode::Char('l') if total_filters > 1 => {
                self.history_unassigned_only = false;
                self.history_space_idx = (self.history_space_idx + 1) % total_filters;
                self.selected_session_idx = 0;
                self.refresh_data();
            }
            KeyCode::Left | KeyCode::Char('h') if total_filters > 1 => {
                self.history_unassigned_only = false;
                self.history_space_idx =
                    (self.history_space_idx + total_filters - 1) % total_filters;
                self.selected_session_idx = 0;
                self.refresh_data();
            }
            KeyCode::Up | KeyCode::Char('k') if !self.sessions.is_empty() => {
                self.selected_session_idx = if self.selected_session_idx == 0 {
                    self.sessions.len() - 1
                } else {
                    self.selected_session_idx - 1
                };
            }
            KeyCode::Down | KeyCode::Char('j') if !self.sessions.is_empty() => {
                self.selected_session_idx = (self.selected_session_idx + 1) % self.sessions.len();
            }
            KeyCode::Char('u') => {
                self.history_unassigned_only = !self.history_unassigned_only;
                if self.history_unassigned_only {
                    self.history_space_idx = 0;
                }
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
                    let short = sess.short_id().to_string();
                    let target = PendingDelete::Session(sid.clone());
                    if self.pending_delete.as_ref() == Some(&target) {
                        self.pending_delete = None;
                        let _ = self.db.delete_session(&sid);
                        self.set_status(format!("Deleted session {}", short));
                        self.refresh_data();
                    } else {
                        self.pending_delete = Some(target);
                        self.set_status(format!("Press 'd' again to delete session {}", short));
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_stats_key(&mut self, key: KeyEvent) {
        let total_filters = self.spaces.len() + 1; // 0 = All, 1..=N = spaces
        match key.code {
            KeyCode::Right | KeyCode::Char('l') if total_filters > 1 => {
                self.stats_space_idx = (self.stats_space_idx + 1) % total_filters;
                self.refresh_data();
            }
            KeyCode::Left | KeyCode::Char('h') if total_filters > 1 => {
                self.stats_space_idx = (self.stats_space_idx + total_filters - 1) % total_filters;
                self.refresh_data();
            }
            KeyCode::Char('r') => {
                self.stats_range = self.stats_range.next();
                self.refresh_data();
            }
            _ => {}
        }
    }

    fn switch_tab(&mut self, next: ActiveTab) {
        self.pending_delete = None;
        if self.active_tab != next {
            self.previous_tab = self.active_tab;
            self.active_tab = next;
        }
    }

    fn handle_settings_key(&mut self, key: KeyEvent) {
        const TOTAL_ITEMS: usize = 11;
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
            KeyCode::Char('e') if self.settings_idx == 10 => {
                self.modal = ModalState::DbPathEditor {
                    buf: Database::display_short_path(self.db.path()),
                };
            }
            KeyCode::Char(' ') | KeyCode::Enter => {
                if self.settings_idx == 10 {
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
            8 => self.config.transparent_background = !self.config.transparent_background,
            9 => {
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
                KeyCode::Esc => (),
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
                }
                KeyCode::Char('b') => {
                    // Quick preset 2: 25m x 2
                    self.timer.set_plan(SessionPlan::new(25, 5, 2));
                    self.set_status("Plan: 25m × 2 (5m break)");
                }
                KeyCode::Char('c') => {
                    // Quick preset 3: 50m x 2
                    self.timer.set_plan(SessionPlan::new(50, 10, 2));
                    self.set_status("Plan: 50m × 2 (10m break)");
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
                }
                _ => self.modal = modal,
            },
            ModalState::NoteEditor { buf } => match key.code {
                KeyCode::Esc => (),
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
                }
                _ => self.modal = modal,
            },
            ModalState::DbPathEditor { buf } => match key.code {
                KeyCode::Esc => (),
                KeyCode::Backspace => {
                    buf.pop();
                    self.modal = modal;
                }
                KeyCode::Char(ch) => {
                    buf.push(ch);
                    self.modal = modal;
                }
                KeyCode::Enter => match self.db.relocate(buf) {
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
                },
                _ => self.modal = modal,
            },
            ModalState::SpaceEditor {
                original_name,
                name_buf,
            } => match key.code {
                KeyCode::Esc => (),
                KeyCode::Backspace => {
                    name_buf.pop();
                    self.modal = modal;
                }
                KeyCode::Char(ch) => {
                    name_buf.push(ch);
                    self.modal = modal;
                }
                KeyCode::Enter => {
                    let name = name_buf.trim().to_string();
                    if !name.is_empty() {
                        let stored_opt = if let Some(ref orig) = original_name {
                            match self.db.rename_space(orig, &name) {
                                Ok(stored) => {
                                    if let Some(ref mut ap) = self.active_project {
                                        if ap
                                            .space
                                            .as_deref()
                                            .map(|s| s.eq_ignore_ascii_case(orig))
                                            .unwrap_or(false)
                                        {
                                            ap.space = Some(stored.clone());
                                        }
                                    }
                                    self.set_status(format!("Renamed space → {}", stored));
                                    Some(stored)
                                }
                                Err(e) => {
                                    self.set_status(format!("Could not rename space: {}", e));
                                    None
                                }
                            }
                        } else {
                            match self.db.add_space(&name) {
                                Ok(stored) => {
                                    self.set_status(format!("Added space: {}", stored));
                                    Some(stored)
                                }
                                Err(e) => {
                                    self.set_status(format!("Could not add space: {}", e));
                                    None
                                }
                            }
                        };
                        if let Some(stored) = stored_opt {
                            self.refresh_data();
                            if let Some(idx) = self
                                .spaces
                                .iter()
                                .position(|s| s.eq_ignore_ascii_case(&stored))
                            {
                                self.selected_space_idx = idx + 1;
                                self.selected_project_idx = 0;
                                self.selected_activity_idx = 0;
                                self.refresh_data();
                            }
                        }
                    }
                }
                _ => self.modal = modal,
            },
            ModalState::ItemEditor {
                edit_id,
                is_activity,
                name_buf,
                space_buf,
                est_buf,
                focus,
            } => match key.code {
                KeyCode::Esc => (),
                KeyCode::Tab | KeyCode::BackTab => {
                    *focus = 1 - *focus;
                    self.modal = modal;
                }
                KeyCode::Down | KeyCode::Right
                    if !*is_activity && *focus == 1 && !self.spaces.is_empty() =>
                {
                    let cur = self
                        .spaces
                        .iter()
                        .position(|s| s.eq_ignore_ascii_case(space_buf.trim()));
                    match cur {
                        None => *space_buf = self.spaces[0].clone(),
                        Some(i) if i + 1 < self.spaces.len() => {
                            *space_buf = self.spaces[i + 1].clone();
                        }
                        Some(_) => space_buf.clear(),
                    }
                    self.modal = modal;
                }
                KeyCode::Up | KeyCode::Left
                    if !*is_activity && *focus == 1 && !self.spaces.is_empty() =>
                {
                    let cur = self
                        .spaces
                        .iter()
                        .position(|s| s.eq_ignore_ascii_case(space_buf.trim()));
                    match cur {
                        None => *space_buf = self.spaces[self.spaces.len() - 1].clone(),
                        Some(0) => space_buf.clear(),
                        Some(i) => *space_buf = self.spaces[i - 1].clone(),
                    }
                    self.modal = modal;
                }
                KeyCode::Up | KeyCode::Right if *is_activity && *focus == 1 => {
                    let cur = est_buf.parse::<u32>().unwrap_or(1);
                    *est_buf = (cur + 1).min(50).to_string();
                    self.modal = modal;
                }
                KeyCode::Down | KeyCode::Left if *is_activity && *focus == 1 => {
                    let cur = est_buf.parse::<u32>().unwrap_or(1);
                    *est_buf = cur.saturating_sub(1).max(1).to_string();
                    self.modal = modal;
                }
                KeyCode::Backspace => {
                    if *focus == 0 {
                        name_buf.pop();
                    } else if *is_activity {
                        est_buf.pop();
                    } else {
                        space_buf.pop();
                    }
                    self.modal = modal;
                }
                KeyCode::Char(ch) => {
                    if *focus == 0 {
                        name_buf.push(ch);
                    } else if *is_activity {
                        if ch.is_ascii_digit() && est_buf.len() < 2 {
                            est_buf.push(ch);
                        }
                    } else {
                        space_buf.push(ch);
                    }
                    self.modal = modal;
                }
                KeyCode::Enter => {
                    let name = name_buf.trim().to_string();
                    if !name.is_empty() {
                        if *is_activity {
                            let est = est_buf.parse::<u32>().unwrap_or(1).max(1);
                            if let Some(ref aid) = edit_id {
                                match self.db.update_activity(aid, &name, est) {
                                    Ok(Some(updated)) => {
                                        if let Some(ref mut aa) = self.active_activity {
                                            if aa.id == updated.id {
                                                *aa = updated.clone();
                                            }
                                        }
                                        self.set_status(format!("Updated activity: {}", name));
                                    }
                                    Ok(None) => {}
                                    Err(e) => {
                                        self.set_status(format!(
                                            "Could not update activity: {}",
                                            e
                                        ));
                                    }
                                }
                            } else if let Some(proj) =
                                self.filtered_projects.get(self.selected_project_idx)
                            {
                                let _ = self.db.get_or_create_activity(&proj.id, &name, est);
                                self.set_status(format!("Added activity: {}", name));
                            }
                        } else {
                            let sp = space_buf.trim();
                            let sp_opt = if sp.is_empty() { None } else { Some(sp) };
                            if let Some(ref pid) = edit_id {
                                match self.db.update_project(pid, &name, sp_opt) {
                                    Ok(Some(updated)) => {
                                        if let Some(ref mut ap) = self.active_project {
                                            if ap.id == updated.id {
                                                *ap = updated.clone();
                                            }
                                        }
                                        self.set_status(format!("Updated project: {}", name));
                                    }
                                    Ok(None) => {}
                                    Err(e) => {
                                        self.set_status(format!("Could not update project: {}", e));
                                    }
                                }
                            } else {
                                let _ = self.db.get_or_create_project_in_space(&name, sp_opt, None);
                                if let Some(s) = sp_opt {
                                    self.set_status(format!("Added project: {} ({})", name, s));
                                } else {
                                    self.set_status(format!("Added project: {}", name));
                                }
                            }
                        }
                        self.refresh_data();
                    }
                }
                _ => self.modal = modal,
            },
            ModalState::ContextPicker {
                session_id,
                focus,
                space_input,
                project_input,
                activity_input,
                status,
                notes_input,
                space_idx,
                project_idx,
                activity_idx,
            } => {
                let is_history = session_id.is_some();
                let max_focus = if is_history { 5 } else { 3 };
                match key.code {
                    KeyCode::Esc => (),
                    KeyCode::Tab => {
                        *focus = (*focus + 1) % max_focus;
                        self.modal = modal;
                    }
                    KeyCode::BackTab => {
                        *focus = (*focus + max_focus - 1) % max_focus;
                        self.modal = modal;
                    }
                    KeyCode::Down if *focus == 0 && !self.spaces.is_empty() => {
                        *space_idx = (*space_idx + 1) % (self.spaces.len() + 1);
                        if *space_idx == 0 {
                            space_input.clear();
                        } else if let Some(sp) = self.spaces.get(*space_idx - 1) {
                            *space_input = sp.clone();
                        }
                        *project_idx = 0;
                        self.modal = modal;
                    }
                    KeyCode::Up if *focus == 0 && !self.spaces.is_empty() => {
                        let len = self.spaces.len() + 1;
                        *space_idx = (*space_idx + len - 1) % len;
                        if *space_idx == 0 {
                            space_input.clear();
                        } else if let Some(sp) = self.spaces.get(*space_idx - 1) {
                            *space_input = sp.clone();
                        }
                        *project_idx = 0;
                        self.modal = modal;
                    }
                    KeyCode::Down if *focus == 1 && !self.projects.is_empty() => {
                        let sp_trim = space_input.trim();
                        let candidates: Vec<&Project> = if sp_trim.is_empty() {
                            self.projects.iter().collect()
                        } else {
                            let in_space: Vec<&Project> = self
                                .projects
                                .iter()
                                .filter(|p| {
                                    p.space
                                        .as_deref()
                                        .map(|s| s.eq_ignore_ascii_case(sp_trim))
                                        .unwrap_or(false)
                                })
                                .collect();
                            if in_space.is_empty() {
                                self.projects.iter().collect()
                            } else {
                                in_space
                            }
                        };
                        *project_idx = (*project_idx + 1) % (candidates.len() + 1);
                        if *project_idx == 0 {
                            project_input.clear();
                        } else if let Some(p) = candidates.get(*project_idx - 1) {
                            *project_input = p.name.clone();
                            if sp_trim.is_empty() {
                                if let Some(ref psp) = p.space {
                                    *space_input = psp.clone();
                                }
                            }
                        }
                        self.modal = modal;
                    }
                    KeyCode::Up if *focus == 1 && !self.projects.is_empty() => {
                        let sp_trim = space_input.trim();
                        let candidates: Vec<&Project> = if sp_trim.is_empty() {
                            self.projects.iter().collect()
                        } else {
                            let in_space: Vec<&Project> = self
                                .projects
                                .iter()
                                .filter(|p| {
                                    p.space
                                        .as_deref()
                                        .map(|s| s.eq_ignore_ascii_case(sp_trim))
                                        .unwrap_or(false)
                                })
                                .collect();
                            if in_space.is_empty() {
                                self.projects.iter().collect()
                            } else {
                                in_space
                            }
                        };
                        let len = candidates.len() + 1;
                        *project_idx = (*project_idx + len - 1) % len;
                        if *project_idx == 0 {
                            project_input.clear();
                        } else if let Some(p) = candidates.get(*project_idx - 1) {
                            *project_input = p.name.clone();
                            if sp_trim.is_empty() {
                                if let Some(ref psp) = p.space {
                                    *space_input = psp.clone();
                                }
                            }
                        }
                        self.modal = modal;
                    }
                    KeyCode::Down if *focus == 2 => {
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
                    KeyCode::Up if *focus == 2 => {
                        if let Ok(Some(proj)) = self.db.find_project_by_name_or_id(project_input) {
                            if let Ok(acts) = self.db.list_activities(Some(&proj.id), false) {
                                if !acts.is_empty() {
                                    let len = acts.len() + 1;
                                    *activity_idx = (*activity_idx + len - 1) % len;
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
                    KeyCode::Left | KeyCode::Right | KeyCode::Char(' ') if *focus == 3 => {
                        *status = status.next_editable();
                        self.modal = modal;
                    }
                    KeyCode::Backspace => {
                        match *focus {
                            0 => {
                                space_input.pop();
                            }
                            1 => {
                                project_input.pop();
                            }
                            2 => {
                                activity_input.pop();
                            }
                            4 => {
                                notes_input.pop();
                            }
                            _ => {}
                        }
                        self.modal = modal;
                    }
                    KeyCode::Char(ch) => {
                        match *focus {
                            0 => space_input.push(ch),
                            1 => project_input.push(ch),
                            2 => activity_input.push(ch),
                            4 => notes_input.push(ch),
                            _ => {}
                        }
                        self.modal = modal;
                    }
                    KeyCode::Enter => {
                        let s_trim = space_input.trim().to_string();
                        let sp_opt = if s_trim.is_empty() {
                            None
                        } else {
                            Some(s_trim.as_str())
                        };
                        let p_trim = project_input.trim().to_string();
                        let a_trim = activity_input.trim().to_string();
                        let resolved_proj = if p_trim.is_empty() {
                            None
                        } else {
                            let proj = self
                                .db
                                .get_or_create_project_in_space(&p_trim, sp_opt, None)
                                .ok();
                            if let Some(mut p) = proj {
                                if sp_opt.is_none() && p.space.is_some() {
                                    let _ = self.db.set_project_space(&p.id, None);
                                    p.space = None;
                                }
                                Some(p)
                            } else {
                                None
                            }
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
        space_filter: None,
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
