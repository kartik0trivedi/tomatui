use crate::config::Config;
use crate::timer::PomodoroPhase;
use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, Utc};
use directories::ProjectDirs;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    InProgress,
    Completed,
    Partial,
    Skipped,
}

impl SessionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
            Self::Partial => "partial",
            Self::Skipped => "skipped",
        }
    }

    pub fn from_db(s: &str) -> Self {
        match s {
            "in_progress" => Self::InProgress,
            "partial" => Self::Partial,
            "skipped" => Self::Skipped,
            _ => Self::Completed,
        }
    }

    pub fn parse_user(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "completed" | "done" => Some(Self::Completed),
            "partial" => Some(Self::Partial),
            "skipped" | "skip" => Some(Self::Skipped),
            "in_progress" | "running" => Some(Self::InProgress),
            _ => None,
        }
    }

    pub fn next_editable(&self) -> Self {
        match self {
            Self::Completed => Self::Partial,
            Self::Partial => Self::Skipped,
            Self::Skipped | Self::InProgress => Self::Completed,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
    pub archived: bool,
    pub created_at: String,
    pub total_focus_mins: u32,
    pub session_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Activity {
    pub id: String,
    pub project_id: String,
    pub project_name: String,
    pub name: String,
    pub estimated_pomodoros: u32,
    pub completed: bool,
    pub created_at: String,
    pub pomodoros_spent: u32,
    pub total_focus_mins: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecord {
    pub id: String,
    pub project_id: Option<String>,
    pub project_name: Option<String>,
    pub activity_id: Option<String>,
    pub activity_name: Option<String>,
    pub phase: PomodoroPhase,
    pub status: SessionStatus,
    pub planned_mins: u32,
    pub actual_secs: u32,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub notes: String,
}

impl SessionRecord {
    pub fn short_id(&self) -> &str {
        &self.id[..self.id.len().min(8)]
    }

    pub fn actual_mins(&self) -> u32 {
        (self.actual_secs + 30) / 60
    }
}

#[derive(Debug, Clone, Default)]
pub struct SessionFilter {
    pub from_date: Option<String>,
    pub to_date: Option<String>,
    pub project: Option<String>,
    pub unassigned_only: bool,
    pub work_only: bool,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatsRange {
    Today,
    Week,
    Month,
    Year,
    All,
}

impl StatsRange {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Today => "Today",
            Self::Week => "7 Days",
            Self::Month => "30 Days",
            Self::Year => "Year",
            Self::All => "All Time",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Self::Today => Self::Week,
            Self::Week => Self::Month,
            Self::Month => Self::All,
            Self::Year => Self::All,
            Self::All => Self::Today,
        }
    }

    pub fn parse_user(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "today" | "1d" => Some(Self::Today),
            "week" | "7d" => Some(Self::Week),
            "month" | "30d" => Some(Self::Month),
            "year" | "365d" => Some(Self::Year),
            "all" => Some(Self::All),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectBreakdown {
    pub project: String,
    pub sessions: u32,
    pub focus_mins: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityBreakdown {
    pub project: String,
    pub activity: String,
    pub sessions: u32,
    pub focus_mins: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DayBucket {
    pub date: String,
    pub label: String,
    pub sessions: u32,
    pub focus_mins: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsReport {
    pub range: StatsRange,
    pub today_sessions: u32,
    pub today_focus_mins: u32,
    pub range_sessions: u32,
    pub range_focus_mins: u32,
    pub current_streak_days: u32,
    pub best_streak_days: u32,
    pub unassigned_sessions: u32,
    pub last_7_days: Vec<DayBucket>,
    pub by_project: Vec<ProjectBreakdown>,
    pub by_activity: Vec<ActivityBreakdown>,
}

pub struct Database {
    conn: Connection,
    path: PathBuf,
}

impl Database {
    pub fn pointer_file_path() -> Option<PathBuf> {
        ProjectDirs::from("com", "tomatui", "tomatui").map(|d| d.config_dir().join("db_path.txt"))
    }

    pub fn platform_default_path() -> PathBuf {
        if let Some(proj_dirs) = ProjectDirs::from("com", "tomatui", "tomatui") {
            return proj_dirs.data_dir().join("tomatui.db");
        }
        if let Some(home) = std::env::var_os("HOME").filter(|h| !h.is_empty()) {
            return PathBuf::from(home).join(".local/share/tomatui/tomatui.db");
        }
        PathBuf::from("./tomatui.db")
    }

    pub fn read_custom_db_path() -> Option<PathBuf> {
        let ptr = Self::pointer_file_path()?;
        let raw = std::fs::read_to_string(ptr).ok()?;
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(Self::expand_tilde(trimmed))
        }
    }

    pub fn save_custom_db_path(path: Option<&Path>) -> std::io::Result<()> {
        let Some(ptr) = Self::pointer_file_path() else {
            return Ok(());
        };
        if let Some(p) = path {
            if let Some(parent) = ptr.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(ptr, p.to_string_lossy().as_bytes())
        } else if ptr.exists() {
            std::fs::remove_file(ptr)
        } else {
            Ok(())
        }
    }

    pub fn expand_tilde(input: &str) -> PathBuf {
        let trimmed = input.trim();
        if trimmed == "~" {
            if let Some(home) = std::env::var_os("HOME") {
                return PathBuf::from(home);
            }
        } else if let Some(rest) = trimmed.strip_prefix("~/") {
            if let Some(home) = std::env::var_os("HOME") {
                return PathBuf::from(home).join(rest);
            }
        }
        PathBuf::from(trimmed)
    }

    pub fn display_short_path(path: &Path) -> String {
        let full = path.to_string_lossy().to_string();
        if let Some(home) = std::env::var_os("HOME") {
            let home_s = home.to_string_lossy();
            if !home_s.is_empty() && full.starts_with(home_s.as_ref()) {
                return format!("~{}", &full[home_s.len()..]);
            }
        }
        full
    }

    pub fn default_path() -> PathBuf {
        if let Ok(env_path) = std::env::var("TOMATUI_DB") {
            if !env_path.trim().is_empty() {
                return Self::expand_tilde(&env_path);
            }
        }
        if let Some(custom) = Self::read_custom_db_path() {
            return custom;
        }
        Self::platform_default_path()
    }

    /// Switches to a new SQLite database path (or resets to platform default if blank).
    /// Flushes WAL and copies existing data when the destination file does not yet exist.
    pub fn relocate(&self, target_input: &str) -> Result<Self, String> {
        let trimmed = target_input.trim();
        let (target, save_ptr) = if trimmed.is_empty() {
            (Self::platform_default_path(), false)
        } else {
            let mut p = Self::expand_tilde(trimmed);
            if p.is_dir() || trimmed.ends_with('/') {
                p = p.join("tomatui.db");
            }
            (p, true)
        };

        if target == self.path {
            return Self::open(&target).map_err(|e| e.to_string());
        }

        if let Some(parent) = target.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
        }

        // Flush WAL into the main DB file before copying so no recent rows are left behind
        let _ = self.conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");

        if !target.exists() && self.path.exists() && self.path != Path::new(":memory:") {
            std::fs::copy(&self.path, &target).map_err(|e| format!("Failed to copy DB: {}", e))?;
        }

        let new_db = Self::open(&target).map_err(|e| format!("Failed to open DB: {}", e))?;
        if save_ptr {
            Self::save_custom_db_path(Some(&target)).map_err(|e| e.to_string())?;
        } else {
            Self::save_custom_db_path(None).map_err(|e| e.to_string())?;
        }
        Ok(new_db)
    }

    pub fn open_default() -> rusqlite::Result<Self> {
        Self::open(Self::default_path())
    }

    pub fn open<P: AsRef<Path>>(path: P) -> rusqlite::Result<Self> {
        let path_buf = path.as_ref().to_path_buf();
        if let Some(parent) = path_buf.parent() {
            if !parent.as_os_str().is_empty() {
                let _ = std::fs::create_dir_all(parent);
            }
        }
        let conn = Connection::open(&path_buf)?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA foreign_keys = ON;
             PRAGMA busy_timeout = 5000;",
        )?;
        let db = Self {
            conn,
            path: path_buf,
        };
        db.migrate()?;
        Ok(db)
    }

    pub fn open_in_memory() -> rusqlite::Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        let db = Self {
            conn,
            path: PathBuf::from(":memory:"),
        };
        db.migrate()?;
        Ok(db)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn migrate(&self) -> rusqlite::Result<()> {
        let version: i32 = self
            .conn
            .query_row("PRAGMA user_version;", [], |row| row.get(0))?;
        if version < 1 {
            self.conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS projects (
                    id          TEXT PRIMARY KEY,
                    name        TEXT NOT NULL UNIQUE COLLATE NOCASE,
                    color       TEXT,
                    archived    INTEGER NOT NULL DEFAULT 0,
                    created_at  TEXT NOT NULL
                );

                CREATE TABLE IF NOT EXISTS activities (
                    id                  TEXT PRIMARY KEY,
                    project_id          TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
                    name                TEXT NOT NULL,
                    estimated_pomodoros INTEGER NOT NULL DEFAULT 1,
                    completed           INTEGER NOT NULL DEFAULT 0,
                    created_at          TEXT NOT NULL,
                    UNIQUE(project_id, name COLLATE NOCASE)
                );

                CREATE TABLE IF NOT EXISTS sessions (
                    id           TEXT PRIMARY KEY,
                    project_id   TEXT REFERENCES projects(id) ON DELETE SET NULL,
                    activity_id  TEXT REFERENCES activities(id) ON DELETE SET NULL,
                    phase        TEXT NOT NULL CHECK (phase IN ('work', 'short_break', 'long_break')),
                    status       TEXT NOT NULL CHECK (status IN ('in_progress', 'completed', 'partial', 'skipped')),
                    planned_mins INTEGER NOT NULL,
                    actual_secs  INTEGER NOT NULL DEFAULT 0,
                    started_at   TEXT NOT NULL,
                    ended_at     TEXT,
                    notes        TEXT NOT NULL DEFAULT ''
                );

                CREATE TABLE IF NOT EXISTS app_config (
                    key   TEXT PRIMARY KEY,
                    value TEXT NOT NULL
                );

                CREATE INDEX IF NOT EXISTS idx_sessions_started_at ON sessions(started_at);
                CREATE INDEX IF NOT EXISTS idx_sessions_project    ON sessions(project_id);
                CREATE INDEX IF NOT EXISTS idx_sessions_activity   ON sessions(activity_id);

                PRAGMA user_version = 1;",
            )?;
        }
        Ok(())
    }

    // --- Config ---

    pub fn load_config(&self) -> Config {
        let raw: Option<String> = self
            .conn
            .query_row(
                "SELECT value FROM app_config WHERE key = 'config'",
                [],
                |row| row.get(0),
            )
            .optional()
            .ok()
            .flatten();

        if let Some(json) = raw {
            if let Ok(mut cfg) = serde_json::from_str::<Config>(&json) {
                cfg.sanitize();
                return cfg;
            }
        }
        Config::default()
    }

    pub fn save_config(&self, config: &Config) -> rusqlite::Result<()> {
        let mut clean = config.clone();
        clean.sanitize();
        let json = serde_json::to_string(&clean).unwrap_or_else(|_| "{}".to_string());
        self.conn.execute(
            "INSERT INTO app_config (key, value) VALUES ('config', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![json],
        )?;
        Ok(())
    }

    // --- Projects ---

    pub fn get_or_create_project(
        &self,
        name: &str,
        color: Option<&str>,
    ) -> rusqlite::Result<Project> {
        let trimmed = name.trim();
        if let Some(existing) = self.find_project_by_name_or_id(trimmed)? {
            return Ok(existing);
        }
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO projects (id, name, color, archived, created_at)
             VALUES (?1, ?2, ?3, 0, ?4)",
            params![id, trimmed, color, now],
        )?;
        Ok(Project {
            id,
            name: trimmed.to_string(),
            color: color.map(ToString::to_string),
            archived: false,
            created_at: now,
            total_focus_mins: 0,
            session_count: 0,
        })
    }

    pub fn find_project_by_name_or_id(&self, query: &str) -> rusqlite::Result<Option<Project>> {
        let q = query.trim();
        let like_prefix = format!("{}%", q);
        let mut stmt = self.conn.prepare(
            "SELECT p.id, p.name, p.color, p.archived, p.created_at,
                    COALESCE(SUM(CASE WHEN s.phase='work' AND s.status IN ('completed','partial') THEN s.actual_secs ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN s.phase='work' AND s.status IN ('completed','partial') THEN 1 ELSE 0 END), 0)
             FROM projects p
             LEFT JOIN sessions s ON s.project_id = p.id
             WHERE p.id = ?1 OR p.name = ?1 COLLATE NOCASE OR p.id LIKE ?2
             GROUP BY p.id
             LIMIT 1",
        )?;
        stmt.query_row(params![q, like_prefix], |row| {
            let secs: i64 = row.get(5)?;
            Ok(Project {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
                archived: row.get::<_, i64>(3)? != 0,
                created_at: row.get(4)?,
                total_focus_mins: ((secs + 30) / 60) as u32,
                session_count: row.get::<_, i64>(6)? as u32,
            })
        })
        .optional()
    }

    pub fn list_projects(&self, include_archived: bool) -> rusqlite::Result<Vec<Project>> {
        let sql = if include_archived {
            "SELECT p.id, p.name, p.color, p.archived, p.created_at,
                    COALESCE(SUM(CASE WHEN s.phase='work' AND s.status IN ('completed','partial') THEN s.actual_secs ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN s.phase='work' AND s.status IN ('completed','partial') THEN 1 ELSE 0 END), 0)
             FROM projects p
             LEFT JOIN sessions s ON s.project_id = p.id
             GROUP BY p.id
             ORDER BY p.archived ASC, p.name COLLATE NOCASE ASC"
        } else {
            "SELECT p.id, p.name, p.color, p.archived, p.created_at,
                    COALESCE(SUM(CASE WHEN s.phase='work' AND s.status IN ('completed','partial') THEN s.actual_secs ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN s.phase='work' AND s.status IN ('completed','partial') THEN 1 ELSE 0 END), 0)
             FROM projects p
             LEFT JOIN sessions s ON s.project_id = p.id
             WHERE p.archived = 0
             GROUP BY p.id
             ORDER BY p.name COLLATE NOCASE ASC"
        };
        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt.query_map([], |row| {
            let secs: i64 = row.get(5)?;
            Ok(Project {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
                archived: row.get::<_, i64>(3)? != 0,
                created_at: row.get(4)?,
                total_focus_mins: ((secs + 30) / 60) as u32,
                session_count: row.get::<_, i64>(6)? as u32,
            })
        })?;
        rows.collect()
    }

    pub fn set_project_archived(&self, query: &str, archived: bool) -> rusqlite::Result<bool> {
        let Some(proj) = self.find_project_by_name_or_id(query)? else {
            return Ok(false);
        };
        self.conn.execute(
            "UPDATE projects SET archived = ?1 WHERE id = ?2",
            params![if archived { 1 } else { 0 }, proj.id],
        )?;
        Ok(true)
    }

    // --- Activities ---

    pub fn get_or_create_activity(
        &self,
        project_id: &str,
        name: &str,
        estimated: u32,
    ) -> rusqlite::Result<Activity> {
        let trimmed = name.trim();
        let existing: Option<String> = self
            .conn
            .query_row(
                "SELECT id FROM activities WHERE project_id = ?1 AND name = ?2 COLLATE NOCASE",
                params![project_id, trimmed],
                |row| row.get(0),
            )
            .optional()?;
        if let Some(id) = existing {
            if let Some(act) = self.get_activity_by_id(&id)? {
                return Ok(act);
            }
        }
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let est = estimated.clamp(1, 50);
        self.conn.execute(
            "INSERT INTO activities (id, project_id, name, estimated_pomodoros, completed, created_at)
             VALUES (?1, ?2, ?3, ?4, 0, ?5)",
            params![id, project_id, trimmed, est, now],
        )?;
        Ok(self.get_activity_by_id(&id)?.expect("inserted activity"))
    }

    pub fn get_activity_by_id(&self, id_or_prefix: &str) -> rusqlite::Result<Option<Activity>> {
        let like_prefix = format!("{}%", id_or_prefix.trim());
        let mut stmt = self.conn.prepare(
            "SELECT a.id, a.project_id, p.name, a.name, a.estimated_pomodoros, a.completed, a.created_at,
                    COALESCE(SUM(CASE WHEN s.phase='work' AND s.status='completed' THEN 1 ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN s.phase='work' AND s.status IN ('completed','partial') THEN s.actual_secs ELSE 0 END), 0)
             FROM activities a
             JOIN projects p ON p.id = a.project_id
             LEFT JOIN sessions s ON s.activity_id = a.id
             WHERE a.id = ?1 OR a.id LIKE ?2
             GROUP BY a.id
             LIMIT 1",
        )?;
        stmt.query_row(params![id_or_prefix.trim(), like_prefix], |row| {
            let secs: i64 = row.get(8)?;
            Ok(Activity {
                id: row.get(0)?,
                project_id: row.get(1)?,
                project_name: row.get(2)?,
                name: row.get(3)?,
                estimated_pomodoros: row.get::<_, i64>(4)? as u32,
                completed: row.get::<_, i64>(5)? != 0,
                created_at: row.get(6)?,
                pomodoros_spent: row.get::<_, i64>(7)? as u32,
                total_focus_mins: ((secs + 30) / 60) as u32,
            })
        })
        .optional()
    }

    pub fn list_activities(
        &self,
        project_id: Option<&str>,
        include_completed: bool,
    ) -> rusqlite::Result<Vec<Activity>> {
        let mut sql = String::from(
            "SELECT a.id, a.project_id, p.name, a.name, a.estimated_pomodoros, a.completed, a.created_at,
                    COALESCE(SUM(CASE WHEN s.phase='work' AND s.status='completed' THEN 1 ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN s.phase='work' AND s.status IN ('completed','partial') THEN s.actual_secs ELSE 0 END), 0)
             FROM activities a
             JOIN projects p ON p.id = a.project_id
             LEFT JOIN sessions s ON s.activity_id = a.id
             WHERE 1=1",
        );
        if project_id.is_some() {
            sql.push_str(" AND a.project_id = ?1");
        }
        if !include_completed {
            sql.push_str(" AND a.completed = 0");
        }
        sql.push_str(" GROUP BY a.id ORDER BY a.completed ASC, a.created_at DESC");

        let mut stmt = self.conn.prepare(&sql)?;
        let map_row = |row: &rusqlite::Row<'_>| -> rusqlite::Result<Activity> {
            let secs: i64 = row.get(8)?;
            Ok(Activity {
                id: row.get(0)?,
                project_id: row.get(1)?,
                project_name: row.get(2)?,
                name: row.get(3)?,
                estimated_pomodoros: row.get::<_, i64>(4)? as u32,
                completed: row.get::<_, i64>(5)? != 0,
                created_at: row.get(6)?,
                pomodoros_spent: row.get::<_, i64>(7)? as u32,
                total_focus_mins: ((secs + 30) / 60) as u32,
            })
        };

        if let Some(pid) = project_id {
            let rows = stmt.query_map(params![pid], map_row)?;
            rows.collect()
        } else {
            let rows = stmt.query_map([], map_row)?;
            rows.collect()
        }
    }

    pub fn toggle_activity_completed(
        &self,
        id_or_prefix: &str,
    ) -> rusqlite::Result<Option<Activity>> {
        let Some(act) = self.get_activity_by_id(id_or_prefix)? else {
            return Ok(None);
        };
        let next = if act.completed { 0 } else { 1 };
        self.conn.execute(
            "UPDATE activities SET completed = ?1 WHERE id = ?2",
            params![next, act.id],
        )?;
        self.get_activity_by_id(&act.id)
    }

    pub fn delete_activity(&self, id_or_prefix: &str) -> rusqlite::Result<bool> {
        let Some(act) = self.get_activity_by_id(id_or_prefix)? else {
            return Ok(false);
        };
        let count = self
            .conn
            .execute("DELETE FROM activities WHERE id = ?1", params![act.id])?;
        Ok(count > 0)
    }

    // --- Sessions ---

    pub fn start_session(
        &self,
        phase: PomodoroPhase,
        planned_mins: u32,
        project_id: Option<&str>,
        activity_id: Option<&str>,
        notes: &str,
    ) -> rusqlite::Result<SessionRecord> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO sessions (id, project_id, activity_id, phase, status, planned_mins, actual_secs, started_at, ended_at, notes)
             VALUES (?1, ?2, ?3, ?4, 'in_progress', ?5, 0, ?6, NULL, ?7)",
            params![id, project_id, activity_id, phase.as_str(), planned_mins, now, notes],
        )?;
        Ok(self.get_session_by_id(&id)?.expect("inserted session"))
    }

    pub fn finish_session(
        &self,
        session_id: &str,
        status: SessionStatus,
        actual_secs: u32,
    ) -> rusqlite::Result<()> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE sessions SET status = ?1, actual_secs = ?2, ended_at = ?3 WHERE id = ?4",
            params![status.as_str(), actual_secs, now, session_id],
        )?;
        Ok(())
    }

    pub fn get_session_by_id(&self, id_or_prefix: &str) -> rusqlite::Result<Option<SessionRecord>> {
        let trimmed = id_or_prefix.trim().trim_start_matches('#');
        let like_prefix = format!("{}%", trimmed);
        let mut stmt = self.conn.prepare(
            "SELECT s.id, s.project_id, p.name, s.activity_id, a.name, s.phase, s.status,
                    s.planned_mins, s.actual_secs, s.started_at, s.ended_at, s.notes
             FROM sessions s
             LEFT JOIN projects p ON p.id = s.project_id
             LEFT JOIN activities a ON a.id = s.activity_id
             WHERE s.id = ?1 OR s.id LIKE ?2
             LIMIT 1",
        )?;
        stmt.query_row(params![trimmed, like_prefix], Self::map_session_row)
            .optional()
    }

    fn map_session_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SessionRecord> {
        Ok(SessionRecord {
            id: row.get(0)?,
            project_id: row.get(1)?,
            project_name: row.get(2)?,
            activity_id: row.get(3)?,
            activity_name: row.get(4)?,
            phase: PomodoroPhase::from_db(&row.get::<_, String>(5)?),
            status: SessionStatus::from_db(&row.get::<_, String>(6)?),
            planned_mins: row.get::<_, i64>(7)? as u32,
            actual_secs: row.get::<_, i64>(8)? as u32,
            started_at: row.get(9)?,
            ended_at: row.get(10)?,
            notes: row.get(11)?,
        })
    }

    pub fn edit_session(
        &self,
        id_or_prefix: &str,
        project_id: Option<Option<&str>>,
        activity_id: Option<Option<&str>>,
        status: Option<SessionStatus>,
        notes: Option<&str>,
    ) -> rusqlite::Result<Option<SessionRecord>> {
        let Some(existing) = self.get_session_by_id(id_or_prefix)? else {
            return Ok(None);
        };
        let new_proj = match project_id {
            Some(v) => v.map(ToString::to_string),
            None => existing.project_id.clone(),
        };
        let new_act = match activity_id {
            Some(v) => v.map(ToString::to_string),
            None => existing.activity_id.clone(),
        };
        let new_status = status.unwrap_or(existing.status);
        let new_notes = notes.unwrap_or(&existing.notes);

        self.conn.execute(
            "UPDATE sessions
             SET project_id = ?1, activity_id = ?2, status = ?3, notes = ?4
             WHERE id = ?5",
            params![
                new_proj,
                new_act,
                new_status.as_str(),
                new_notes,
                existing.id
            ],
        )?;
        self.get_session_by_id(&existing.id)
    }

    pub fn delete_session(&self, id_or_prefix: &str) -> rusqlite::Result<bool> {
        let Some(existing) = self.get_session_by_id(id_or_prefix)? else {
            return Ok(false);
        };
        let count = self
            .conn
            .execute("DELETE FROM sessions WHERE id = ?1", params![existing.id])?;
        Ok(count > 0)
    }

    pub fn list_sessions(&self, filter: &SessionFilter) -> rusqlite::Result<Vec<SessionRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT s.id, s.project_id, p.name, s.activity_id, a.name, s.phase, s.status,
                    s.planned_mins, s.actual_secs, s.started_at, s.ended_at, s.notes
             FROM sessions s
             LEFT JOIN projects p ON p.id = s.project_id
             LEFT JOIN activities a ON a.id = s.activity_id
             ORDER BY s.started_at DESC",
        )?;
        let all: Vec<SessionRecord> = stmt
            .query_map([], Self::map_session_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        let from_d = filter
            .from_date
            .as_deref()
            .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok());
        let to_d = filter
            .to_date
            .as_deref()
            .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok());

        let mut filtered: Vec<SessionRecord> = all
            .into_iter()
            .filter(|s| {
                if filter.work_only && s.phase != PomodoroPhase::Work {
                    return false;
                }
                if filter.unassigned_only && s.project_id.is_some() {
                    return false;
                }
                if let Some(ref proj_q) = filter.project {
                    let matches = s
                        .project_name
                        .as_deref()
                        .map(|n| n.eq_ignore_ascii_case(proj_q))
                        .unwrap_or(false)
                        || s.project_id.as_deref() == Some(proj_q.as_str());
                    if !matches {
                        return false;
                    }
                }
                if from_d.is_some() || to_d.is_some() {
                    if let Ok(dt) = DateTime::parse_from_rfc3339(&s.started_at) {
                        let local_d = dt.with_timezone(&Local).date_naive();
                        if let Some(fd) = from_d {
                            if local_d < fd {
                                return false;
                            }
                        }
                        if let Some(td) = to_d {
                            if local_d > td {
                                return false;
                            }
                        }
                    }
                }
                true
            })
            .collect();

        if let Some(limit) = filter.limit {
            filtered.truncate(limit);
        }
        Ok(filtered)
    }

    // --- Analytics ---

    pub fn get_stats(
        &self,
        range: StatsRange,
        project_filter: Option<&str>,
    ) -> rusqlite::Result<StatsReport> {
        let all_work = self.list_sessions(&SessionFilter {
            project: project_filter.map(ToString::to_string),
            work_only: true,
            ..Default::default()
        })?;

        let today = Local::now().date_naive();
        let range_start: Option<NaiveDate> = match range {
            StatsRange::Today => Some(today),
            StatsRange::Week => Some(today - Duration::days(6)),
            StatsRange::Month => Some(today - Duration::days(29)),
            StatsRange::Year => Some(today - Duration::days(364)),
            StatsRange::All => None,
        };

        let mut today_sessions = 0u32;
        let mut today_secs = 0u32;
        let mut range_sessions = 0u32;
        let mut range_secs = 0u32;
        let mut unassigned_sessions = 0u32;

        let mut distinct_dates: Vec<NaiveDate> = Vec::new();
        let mut day_map: BTreeMap<NaiveDate, (u32, u32)> = BTreeMap::new();
        for i in (0..7).rev() {
            day_map.insert(today - Duration::days(i), (0, 0));
        }
        let mut proj_map: BTreeMap<String, (u32, u32)> = BTreeMap::new();
        let mut act_map: BTreeMap<(String, String), (u32, u32)> = BTreeMap::new();

        for s in &all_work {
            if !matches!(s.status, SessionStatus::Completed | SessionStatus::Partial) {
                continue;
            }
            if s.project_id.is_none() {
                unassigned_sessions += 1;
            }
            let Ok(dt) = DateTime::parse_from_rfc3339(&s.started_at) else {
                continue;
            };
            let d = dt.with_timezone(&Local).date_naive();
            distinct_dates.push(d);

            if d == today {
                today_sessions += 1;
                today_secs = today_secs.saturating_add(s.actual_secs);
            }
            if let Some(entry) = day_map.get_mut(&d) {
                entry.0 += 1;
                entry.1 = entry.1.saturating_add(s.actual_secs);
            }

            let in_range = range_start.map(|rs| d >= rs).unwrap_or(true);
            if in_range {
                range_sessions += 1;
                range_secs = range_secs.saturating_add(s.actual_secs);

                let pname = s
                    .project_name
                    .clone()
                    .unwrap_or_else(|| "Unassigned".to_string());
                let pe = proj_map.entry(pname.clone()).or_insert((0, 0));
                pe.0 += 1;
                pe.1 = pe.1.saturating_add(s.actual_secs);

                if let Some(aname) = &s.activity_name {
                    let ae = act_map.entry((pname, aname.clone())).or_insert((0, 0));
                    ae.0 += 1;
                    ae.1 = ae.1.saturating_add(s.actual_secs);
                }
            }
        }

        distinct_dates.sort();
        distinct_dates.dedup();
        let (current_streak_days, best_streak_days) = compute_streaks(&distinct_dates, today);

        let last_7_days: Vec<DayBucket> = day_map
            .into_iter()
            .map(|(d, (cnt, secs))| {
                let wd = match d.weekday() {
                    chrono::Weekday::Mon => "Mon",
                    chrono::Weekday::Tue => "Tue",
                    chrono::Weekday::Wed => "Wed",
                    chrono::Weekday::Thu => "Thu",
                    chrono::Weekday::Fri => "Fri",
                    chrono::Weekday::Sat => "Sat",
                    chrono::Weekday::Sun => "Sun",
                };
                DayBucket {
                    date: d.format("%Y-%m-%d").to_string(),
                    label: format!("{} {:02}", wd, d.day()),
                    sessions: cnt,
                    focus_mins: (secs + 30) / 60,
                }
            })
            .collect();

        let mut by_project: Vec<ProjectBreakdown> = proj_map
            .into_iter()
            .map(|(project, (sessions, secs))| ProjectBreakdown {
                project,
                sessions,
                focus_mins: (secs + 30) / 60,
            })
            .collect();
        by_project.sort_by_key(|b| std::cmp::Reverse(b.focus_mins));

        let mut by_activity: Vec<ActivityBreakdown> = act_map
            .into_iter()
            .map(
                |((project, activity), (sessions, secs))| ActivityBreakdown {
                    project,
                    activity,
                    sessions,
                    focus_mins: (secs + 30) / 60,
                },
            )
            .collect();
        by_activity.sort_by_key(|b| std::cmp::Reverse(b.focus_mins));

        Ok(StatsReport {
            range,
            today_sessions,
            today_focus_mins: (today_secs + 30) / 60,
            range_sessions,
            range_focus_mins: (range_secs + 30) / 60,
            current_streak_days,
            best_streak_days,
            unassigned_sessions,
            last_7_days,
            by_project,
            by_activity,
        })
    }

    pub fn export_csv(&self, filter: &SessionFilter) -> rusqlite::Result<String> {
        let sessions = self.list_sessions(filter)?;
        let mut out = String::from(
            "id,started_at,ended_at,phase,status,planned_mins,actual_secs,project,activity,notes\n",
        );
        for s in sessions {
            let esc = |v: &str| {
                if v.contains(',') || v.contains('"') || v.contains('\n') {
                    format!("\"{}\"", v.replace('"', "\"\""))
                } else {
                    v.to_string()
                }
            };
            out.push_str(&format!(
                "{},{},{},{},{},{},{},{},{},{}\n",
                esc(&s.id),
                esc(&s.started_at),
                esc(s.ended_at.as_deref().unwrap_or("")),
                s.phase.as_str(),
                s.status.as_str(),
                s.planned_mins,
                s.actual_secs,
                esc(s.project_name.as_deref().unwrap_or("")),
                esc(s.activity_name.as_deref().unwrap_or("")),
                esc(&s.notes),
            ));
        }
        Ok(out)
    }
}

fn compute_streaks(dates: &[NaiveDate], today: NaiveDate) -> (u32, u32) {
    if dates.is_empty() {
        return (0, 0);
    }
    let mut best = 1u32;
    let mut run = 1u32;
    for i in 1..dates.len() {
        if dates[i] == dates[i - 1] + Duration::days(1) {
            run += 1;
            best = best.max(run);
        } else {
            run = 1;
        }
    }

    let yesterday = today - Duration::days(1);
    let last = (*dates.last().unwrap()).min(today);
    if last != today && last != yesterday {
        return (0, best);
    }

    let mut current = 0u32;
    let mut expected = last;
    for &d in dates.iter().rev() {
        if d > today {
            continue;
        }
        if d == expected {
            current += 1;
            expected -= Duration::days(1);
        } else if d < expected {
            break;
        }
    }
    (current, best)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_activity_session_lifecycle_and_edit() {
        let db = Database::open_in_memory().unwrap();
        let proj = db.get_or_create_project("backend-api", None).unwrap();
        let act = db
            .get_or_create_activity(&proj.id, "API schema", 4)
            .unwrap();

        // Start unassigned session, finish it, then edit it post-hoc to assign project + activity
        let sess = db
            .start_session(PomodoroPhase::Work, 25, None, None, "")
            .unwrap();
        assert!(sess.project_id.is_none());
        db.finish_session(&sess.id, SessionStatus::Completed, 1500)
            .unwrap();

        let edited = db
            .edit_session(
                sess.short_id(),
                Some(Some(&proj.id)),
                Some(Some(&act.id)),
                None,
                Some("Auth endpoints"),
            )
            .unwrap()
            .unwrap();

        assert_eq!(edited.project_name.as_deref(), Some("backend-api"));
        assert_eq!(edited.activity_name.as_deref(), Some("API schema"));
        assert_eq!(edited.notes, "Auth endpoints");

        let stats = db.get_stats(StatsRange::Week, None).unwrap();
        assert_eq!(stats.today_sessions, 1);
        assert_eq!(stats.today_focus_mins, 25);
        assert_eq!(stats.unassigned_sessions, 0);
        assert_eq!(stats.by_project.len(), 1);
        assert_eq!(stats.by_project[0].project, "backend-api");
    }
}
