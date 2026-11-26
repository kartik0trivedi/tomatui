use crate::db::{Database, ImportPayload, SessionFilter, SessionStatus, StatsRange};
use clap::{Parser, Subcommand};
use std::io::Read;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct LaunchOptions {
    pub space: Option<String>,
    pub project: Option<String>,
    pub activity: Option<String>,
    pub work_mins: Option<u32>,
    pub break_mins: Option<u32>,
    pub batches: Option<u32>,
    pub autostart: bool,
    pub exit_on_done: bool,
}

#[derive(Debug, Parser)]
#[command(
    name = "tomatui",
    version,
    about = "Clean Pomodoro TUI & agent-friendly CLI backed by SQLite.\nQuick launch: tomatui PROJECT_NAME --25 --2\nAI agents:    tomatui guide --json"
)]
pub struct Cli {
    /// Output structured JSON (ideal for AI agents and scripts)
    #[arg(long, global = true)]
    pub json: bool,

    /// Custom SQLite database path
    #[arg(long, global = true)]
    pub db: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Start a Pomodoro session or batch in the TUI
    Start {
        /// Optional project name (auto-created if new)
        project: Option<String>,
        /// Optional space / group for the project (e.g. School, Work, Client, Personal)
        #[arg(long)]
        space: Option<String>,
        /// Focus duration in minutes
        #[arg(short = 'd', long = "duration")]
        duration: Option<u32>,
        /// Number of Pomodoro batches
        #[arg(short = 'n', long = "batches")]
        batches: Option<u32>,
        /// Break duration in minutes (default: 5)
        #[arg(short = 'b', long = "break")]
        break_mins: Option<u32>,
        /// Activity within the project
        #[arg(short = 'a', long = "activity")]
        activity: Option<String>,
        /// Open in standby mode instead of starting immediately
        #[arg(long)]
        no_autostart: bool,
        /// Exit TUI automatically when all batches finish
        #[arg(long)]
        exit_on_done: bool,
    },
    /// Manage spaces (high-level groups above projects: teams, clients, orgs)
    Spaces {
        #[command(subcommand)]
        action: SpaceCmd,
    },
    /// Manage projects
    Projects {
        #[command(subcommand)]
        action: ProjectCmd,
    },
    /// Manage activities within projects
    Activities {
        #[command(subcommand)]
        action: ActivityCmd,
    },
    /// Bulk-import spaces, projects, and activities from JSON (file or stdin)
    Import {
        /// Path to JSON file (omit or pass '-' to read from stdin)
        file: Option<PathBuf>,
        /// Validate and preview counts without writing to SQLite
        #[arg(long)]
        dry_run: bool,
    },
    /// Print a complete command & schema guide for AI agents and scripts
    #[command(visible_aliases = ["agent-help", "schema"])]
    Guide,
    /// Query and edit recorded sessions
    Sessions {
        #[command(subcommand)]
        action: SessionCmd,
    },
    /// View focus statistics and breakdowns
    Stats {
        /// Range: today, week, month, year, all
        #[arg(long, default_value = "week")]
        range: String,
        /// Filter by space name
        #[arg(long)]
        space: Option<String>,
        /// Filter by project name
        #[arg(long)]
        project: Option<String>,
    },
    /// Export session history as JSON or CSV
    Export {
        #[arg(long, default_value = "json")]
        format: String,
        #[arg(long)]
        from: Option<String>,
        #[arg(long)]
        to: Option<String>,
        #[arg(long)]
        space: Option<String>,
        #[arg(long)]
        project: Option<String>,
    },
    /// Print the active SQLite database path
    DbPath,
}

#[derive(Debug, Subcommand)]
pub enum SpaceCmd {
    List,
    Add { name: String },
    Delete { name: String },
}

#[derive(Debug, Subcommand)]
pub enum ProjectCmd {
    List {
        #[arg(long)]
        archived: bool,
    },
    Add {
        name: String,
        #[arg(long)]
        space: Option<String>,
        #[arg(long)]
        color: Option<String>,
    },
    Archive {
        project: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum ActivityCmd {
    List {
        #[arg(long)]
        project: Option<String>,
        #[arg(long)]
        all: bool,
    },
    Add {
        project: String,
        name: String,
        #[arg(long, default_value_t = 1)]
        est: u32,
    },
    Done {
        id: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum SessionCmd {
    List {
        #[arg(long)]
        from: Option<String>,
        #[arg(long)]
        to: Option<String>,
        #[arg(long)]
        space: Option<String>,
        #[arg(long)]
        project: Option<String>,
        #[arg(long)]
        unassigned: bool,
        #[arg(long)]
        limit: Option<usize>,
    },
    Edit {
        /// Session ID or unique prefix (e.g. a4f91c20)
        id: String,
        #[arg(long)]
        project: Option<String>,
        #[arg(long)]
        activity: Option<String>,
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        notes: Option<String>,
        /// Clear project & activity assignment
        #[arg(long)]
        unassign: bool,
    },
    Delete {
        id: String,
    },
}

const KNOWN_SUBCOMMANDS: &[&str] = &[
    "start",
    "spaces",
    "projects",
    "activities",
    "sessions",
    "stats",
    "import",
    "export",
    "guide",
    "agent-help",
    "schema",
    "db-path",
    "help",
];

/// Rewrites ergonomic shorthand like `tomatui PROJECT --25 --2` into
/// `tomatui start PROJECT --duration 25 --batches 2` before clap parses it.
pub fn normalize_shorthand_args<I, S>(args: I) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let raw: Vec<String> = args.into_iter().map(Into::into).collect();
    if raw.len() <= 1 {
        return raw;
    }

    // Check if any token after argv[0] is a known subcommand or --help/-h/--version/-V
    let mut idx = 1;
    let mut saw_subcommand = false;
    let mut saw_shorthand_trigger = false;

    while idx < raw.len() {
        let tok = &raw[idx];
        if matches!(tok.as_str(), "-h" | "--help" | "-V" | "--version") {
            return raw;
        }
        if tok == "--db" {
            idx += 2;
            continue;
        }
        if tok == "--json" {
            idx += 1;
            continue;
        }
        if KNOWN_SUBCOMMANDS.contains(&tok.as_str()) {
            saw_subcommand = true;
            break;
        }
        // Any positional project name or numeric flag like --25 triggers shorthand start
        if !tok.starts_with('-') || parse_numeric_flag(tok).is_some() {
            saw_shorthand_trigger = true;
        }
        idx += 1;
    }

    if saw_subcommand || !saw_shorthand_trigger {
        return raw;
    }

    let mut out = Vec::with_capacity(raw.len() + 4);
    out.push(raw[0].clone());

    let mut global_flags = Vec::new();
    let mut start_args = Vec::new();
    let mut numeric_count = 0usize;

    let mut i = 1;
    while i < raw.len() {
        let tok = &raw[i];
        if tok == "--db" && i + 1 < raw.len() {
            global_flags.push(tok.clone());
            global_flags.push(raw[i + 1].clone());
            i += 2;
            continue;
        }
        if tok == "--json" {
            global_flags.push(tok.clone());
            i += 1;
            continue;
        }
        if let Some(num) = parse_numeric_flag(tok) {
            numeric_count += 1;
            if numeric_count == 1 {
                start_args.push("--duration".to_string());
                start_args.push(num.to_string());
            } else if numeric_count == 2 {
                start_args.push("--batches".to_string());
                start_args.push(num.to_string());
            } else if numeric_count == 3 {
                start_args.push("--break".to_string());
                start_args.push(num.to_string());
            }
            i += 1;
            continue;
        }
        start_args.push(tok.clone());
        i += 1;
    }

    out.extend(global_flags);
    out.push("start".to_string());
    out.extend(start_args);
    out
}

fn parse_numeric_flag(tok: &str) -> Option<u32> {
    let rest = tok.strip_prefix("--")?;
    if rest.is_empty() || !rest.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    rest.parse::<u32>().ok()
}

pub enum CliDispatch {
    LaunchTui(LaunchOptions),
    Handled,
}

pub fn execute_cli(cli: Cli, db: &Database) -> Result<CliDispatch, String> {
    let json = cli.json;
    match cli.command {
        None => Ok(CliDispatch::LaunchTui(LaunchOptions {
            space: None,
            project: None,
            activity: None,
            work_mins: None,
            break_mins: None,
            batches: None,
            autostart: false,
            exit_on_done: false,
        })),
        Some(Command::Start {
            project,
            space,
            duration,
            batches,
            break_mins,
            activity,
            no_autostart,
            exit_on_done,
        }) => Ok(CliDispatch::LaunchTui(LaunchOptions {
            space,
            project,
            activity,
            work_mins: duration,
            break_mins,
            batches,
            autostart: !no_autostart,
            exit_on_done,
        })),
        Some(Command::DbPath) => {
            let p = db.path().display().to_string();
            if json {
                println!("{}", serde_json::json!({ "db_path": p }));
            } else {
                println!("{}", p);
            }
            Ok(CliDispatch::Handled)
        }
        Some(Command::Spaces { action }) => {
            match action {
                SpaceCmd::List => {
                    let spaces = db.list_spaces().map_err(|e| e.to_string())?;
                    if json {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&serde_json::json!({ "spaces": spaces }))
                                .unwrap()
                        );
                    } else if spaces.is_empty() {
                        println!("No spaces yet. Create one with `tomatui spaces add <NAME>`.");
                    } else {
                        for sp in spaces {
                            println!("{}", sp);
                        }
                    }
                }
                SpaceCmd::Add { name } => {
                    let stored = db.add_space(&name).map_err(|e| e.to_string())?;
                    if json {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&serde_json::json!({ "space": stored }))
                                .unwrap()
                        );
                    } else {
                        println!("Space ready: {}", stored);
                    }
                }
                SpaceCmd::Delete { name } => {
                    let deleted = db.delete_space(&name).map_err(|e| e.to_string())?;
                    if json {
                        println!(
                            "{}",
                            serde_json::json!({ "deleted": deleted, "space": name })
                        );
                    } else if deleted {
                        println!("Removed space: {} (projects kept ungrouped)", name);
                    } else {
                        return Err(format!("Space not found: {}", name));
                    }
                }
            }
            Ok(CliDispatch::Handled)
        }
        Some(Command::Import { file, dry_run }) => {
            let raw = match file {
                Some(ref path) if path.as_os_str() != "-" => std::fs::read_to_string(path)
                    .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?,
                _ => {
                    let mut buf = String::new();
                    std::io::stdin()
                        .read_to_string(&mut buf)
                        .map_err(|e| format!("Failed to read stdin: {}", e))?;
                    buf
                }
            };
            let payload: ImportPayload = serde_json::from_str(&raw)
                .map_err(|e| format!("Invalid import JSON: {}", e))?;
            let summary = db
                .bulk_import(payload, dry_run)
                .map_err(|e| e.to_string())?;
            if json {
                println!("{}", serde_json::to_string_pretty(&summary).unwrap());
            } else {
                let prefix = if summary.dry_run { "[dry-run] " } else { "" };
                println!(
                    "{}Imported {} space(s), {} project(s), {} activity(ies).",
                    prefix,
                    summary.spaces_processed,
                    summary.projects_processed,
                    summary.activities_processed
                );
            }
            Ok(CliDispatch::Handled)
        }
        Some(Command::Guide) => {
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&agent_guide_json(db)).unwrap()
                );
            } else {
                print!("{}", agent_guide_markdown(db));
            }
            Ok(CliDispatch::Handled)
        }
        Some(Command::Projects { action }) => {
            match action {
                ProjectCmd::List { archived } => {
                    let projects = db.list_projects(archived).map_err(|e| e.to_string())?;
                    if json {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(
                                &serde_json::json!({ "projects": projects })
                            )
                            .unwrap()
                        );
                    } else if projects.is_empty() {
                        println!("No projects yet. Create one with `tomatui projects add <NAME>`.");
                    } else {
                        for p in projects {
                            let tag = if p.archived { " [archived]" } else { "" };
                            let sp = p.space.as_deref().unwrap_or("-");
                            println!(
                                "{:<14} {:<24} {:>4} sessions  {:>5}m{}",
                                sp, p.name, p.session_count, p.total_focus_mins, tag
                            );
                        }
                    }
                }
                ProjectCmd::Add { name, space, color } => {
                    let proj = db
                        .get_or_create_project_in_space(&name, space.as_deref(), color.as_deref())
                        .map_err(|e| e.to_string())?;
                    if json {
                        println!("{}", serde_json::to_string_pretty(&proj).unwrap());
                    } else if let Some(ref sp) = proj.space {
                        println!("Project ready: {} / {} ({})", sp, proj.name, &proj.id[..8]);
                    } else {
                        println!("Project ready: {} ({})", proj.name, &proj.id[..8]);
                    }
                }
                ProjectCmd::Archive { project } => {
                    let ok = db
                        .set_project_archived(&project, true)
                        .map_err(|e| e.to_string())?;
                    if json {
                        println!(
                            "{}",
                            serde_json::json!({ "archived": ok, "project": project })
                        );
                    } else if ok {
                        println!("Archived project: {}", project);
                    } else {
                        return Err(format!("Project not found: {}", project));
                    }
                }
            }
            Ok(CliDispatch::Handled)
        }
        Some(Command::Activities { action }) => {
            match action {
                ActivityCmd::List { project, all } => {
                    let proj_id = if let Some(ref p) = project {
                        Some(
                            db.find_project_by_name_or_id(p)
                                .map_err(|e| e.to_string())?
                                .ok_or_else(|| format!("Project not found: {}", p))?
                                .id,
                        )
                    } else {
                        None
                    };
                    let acts = db
                        .list_activities(proj_id.as_deref(), all)
                        .map_err(|e| e.to_string())?;
                    if json {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(
                                &serde_json::json!({ "activities": acts })
                            )
                            .unwrap()
                        );
                    } else if acts.is_empty() {
                        println!("No activities found.");
                    } else {
                        for a in acts {
                            let mark = if a.completed { "[x]" } else { "[ ]" };
                            println!(
                                "{} {}  {:<18} {:<26} {}/{} ({}m)",
                                &a.id[..8],
                                mark,
                                a.project_name,
                                a.name,
                                a.pomodoros_spent,
                                a.estimated_pomodoros,
                                a.total_focus_mins
                            );
                        }
                    }
                }
                ActivityCmd::Add { project, name, est } => {
                    let proj = db
                        .get_or_create_project(&project, None)
                        .map_err(|e| e.to_string())?;
                    let act = db
                        .get_or_create_activity(&proj.id, &name, est)
                        .map_err(|e| e.to_string())?;
                    if json {
                        println!("{}", serde_json::to_string_pretty(&act).unwrap());
                    } else {
                        println!(
                            "Activity ready: {} / {} ({})",
                            proj.name,
                            act.name,
                            &act.id[..8]
                        );
                    }
                }
                ActivityCmd::Done { id } => {
                    let updated = db
                        .toggle_activity_completed(&id)
                        .map_err(|e| e.to_string())?
                        .ok_or_else(|| format!("Activity not found: {}", id))?;
                    if json {
                        println!("{}", serde_json::to_string_pretty(&updated).unwrap());
                    } else {
                        println!(
                            "Activity {} marked {}",
                            updated.name,
                            if updated.completed { "done" } else { "active" }
                        );
                    }
                }
            }
            Ok(CliDispatch::Handled)
        }
        Some(Command::Sessions { action }) => {
            match action {
                SessionCmd::List {
                    from,
                    to,
                    space,
                    project,
                    unassigned,
                    limit,
                } => {
                    let sessions = db
                        .list_sessions(&SessionFilter {
                            from_date: from,
                            to_date: to,
                            space,
                            project,
                            unassigned_only: unassigned,
                            work_only: false,
                            limit,
                        })
                        .map_err(|e| e.to_string())?;
                    if json {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(
                                &serde_json::json!({ "sessions": sessions })
                            )
                            .unwrap()
                        );
                    } else if sessions.is_empty() {
                        println!("No sessions recorded.");
                    } else {
                        for s in sessions {
                            println!(
                                "{}  {}  {:>3}m  {:<10}  {:<12}  {:<16}  {:<20}  {}",
                                s.short_id(),
                                &s.started_at[..s.started_at.len().min(16)],
                                s.actual_mins(),
                                s.status.as_str(),
                                s.space.as_deref().unwrap_or("-"),
                                s.project_name.as_deref().unwrap_or("Unassigned"),
                                s.activity_name.as_deref().unwrap_or("-"),
                                s.notes
                            );
                        }
                    }
                }
                SessionCmd::Edit {
                    id,
                    project,
                    activity,
                    status,
                    notes,
                    unassign,
                } => {
                    let proj_opt = if unassign {
                        Some(None)
                    } else if let Some(ref p_name) = project {
                        let p = db
                            .get_or_create_project(p_name, None)
                            .map_err(|e| e.to_string())?;
                        Some(Some(p.id))
                    } else {
                        None
                    };

                    let act_opt = if unassign {
                        Some(None)
                    } else if let Some(ref a_name) = activity {
                        let existing_sess = db
                            .get_session_by_id(&id)
                            .map_err(|e| e.to_string())?
                            .ok_or_else(|| format!("Session not found: {}", id))?;
                        let pid = proj_opt
                            .as_ref()
                            .and_then(|o| o.clone())
                            .or(existing_sess.project_id)
                            .ok_or_else(|| {
                                "Specify --project when assigning an activity to an unassigned session"
                                    .to_string()
                            })?;
                        let act = db
                            .get_or_create_activity(&pid, a_name, 1)
                            .map_err(|e| e.to_string())?;
                        Some(Some(act.id))
                    } else {
                        None
                    };

                    let status_opt = match status {
                        Some(ref s) => Some(
                            SessionStatus::parse_user(s)
                                .ok_or_else(|| format!("Invalid status: {}", s))?,
                        ),
                        None => None,
                    };

                    let updated = db
                        .edit_session(
                            &id,
                            proj_opt.as_ref().map(|o| o.as_deref()),
                            act_opt.as_ref().map(|o| o.as_deref()),
                            status_opt,
                            notes.as_deref(),
                        )
                        .map_err(|e| e.to_string())?
                        .ok_or_else(|| format!("Session not found: {}", id))?;

                    if json {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&serde_json::json!({
                                "updated": true,
                                "session": updated
                            }))
                            .unwrap()
                        );
                    } else {
                        println!(
                            "Updated session {} -> {} / {}",
                            updated.short_id(),
                            updated.project_name.as_deref().unwrap_or("Unassigned"),
                            updated.activity_name.as_deref().unwrap_or("-")
                        );
                    }
                }
                SessionCmd::Delete { id } => {
                    let deleted = db.delete_session(&id).map_err(|e| e.to_string())?;
                    if json {
                        println!("{}", serde_json::json!({ "deleted": deleted, "id": id }));
                    } else if deleted {
                        println!("Deleted session {}", id);
                    } else {
                        return Err(format!("Session not found: {}", id));
                    }
                }
            }
            Ok(CliDispatch::Handled)
        }
        Some(Command::Stats {
            range,
            space,
            project,
        }) => {
            let r = StatsRange::parse_user(&range).ok_or_else(|| {
                format!("Invalid range '{}'. Use today|week|month|year|all.", range)
            })?;
            let report = db
                .get_stats(r, project.as_deref(), space.as_deref())
                .map_err(|e| e.to_string())?;
            if json {
                println!("{}", serde_json::to_string_pretty(&report).unwrap());
            } else {
                if let Some(ref sp) = report.space_filter {
                    println!("Range: {} | Space: {}", report.range.label(), sp);
                } else {
                    println!("Range: {}", report.range.label());
                }
                println!(
                    "Today: {} sessions ({}m) | Streak: {}d (best {}d)",
                    report.today_sessions,
                    report.today_focus_mins,
                    report.current_streak_days,
                    report.best_streak_days
                );
                println!(
                    "Period total: {} sessions ({}m) | Unassigned: {}",
                    report.range_sessions, report.range_focus_mins, report.unassigned_sessions
                );
                if !report.by_project.is_empty() {
                    println!("\nBy project:");
                    for p in &report.by_project {
                        let label = match p.space.as_deref() {
                            Some(sp) => format!("{} / {}", sp, p.project),
                            None => p.project.clone(),
                        };
                        println!(
                            "  {:<28} {:>3} sessions  {:>5}m",
                            label, p.sessions, p.focus_mins
                        );
                    }
                }
            }
            Ok(CliDispatch::Handled)
        }
        Some(Command::Export {
            format,
            from,
            to,
            space,
            project,
        }) => {
            let filter = SessionFilter {
                from_date: from,
                to_date: to,
                space,
                project,
                unassigned_only: false,
                work_only: false,
                limit: None,
            };
            match format.to_ascii_lowercase().as_str() {
                "csv" => {
                    let csv = db.export_csv(&filter).map_err(|e| e.to_string())?;
                    print!("{}", csv);
                }
                _ => {
                    let sessions = db.list_sessions(&filter).map_err(|e| e.to_string())?;
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&serde_json::json!({ "sessions": sessions }))
                            .unwrap()
                    );
                }
            }
            Ok(CliDispatch::Handled)
        }
    }
}

pub fn agent_guide_json(db: &Database) -> serde_json::Value {
    serde_json::json!({
        "name": "tomatui",
        "version": env!("CARGO_PKG_VERSION"),
        "description": "Offline-first Pomodoro TUI & agent-friendly CLI backed by SQLite.",
        "db_path": db.path().display().to_string(),
        "hierarchy": ["Space (optional)", "Project", "Activity", "Session"],
        "global_flags": [
            { "flag": "--json", "description": "Emit structured JSON on all non-TUI commands" },
            { "flag": "--db <PATH>", "description": "Override SQLite database path (or set TOMATUI_DB)" }
        ],
        "commands": [
            {
                "command": "tomatui guide --json",
                "aliases": ["agent-help", "schema"],
                "description": "Print this machine-readable command & schema manifest (omit --json for Markdown)"
            },
            {
                "command": "tomatui spaces list|add|delete",
                "examples": [
                    "tomatui spaces list --json",
                    "tomatui spaces add Work --json",
                    "tomatui spaces delete Work --json"
                ]
            },
            {
                "command": "tomatui projects list|add|archive",
                "examples": [
                    "tomatui projects list --json",
                    "tomatui projects add backend-api --space Work --json",
                    "tomatui projects archive backend-api --json"
                ]
            },
            {
                "command": "tomatui activities list|add|done",
                "examples": [
                    "tomatui activities list --project backend-api --json",
                    "tomatui activities add backend-api \"API schema\" --est 4 --json",
                    "tomatui activities done <ID_PREFIX> --json"
                ]
            },
            {
                "command": "tomatui import [FILE|-] [--dry-run] [--json]",
                "description": "Idempotently bulk-create spaces, projects, and activities in one SQLite transaction.",
                "examples": [
                    "tomatui import seed.json --json",
                    "cat seed.json | tomatui import --dry-run --json"
                ],
                "accepted_json_shapes": {
                    "bundle": {
                        "spaces": ["School", "Work", "Client", "Personal"],
                        "projects": [
                            {
                                "name": "backend-api",
                                "space": "Work",
                                "color": "#89b4fa",
                                "activities": [
                                    "API schema",
                                    { "name": "Auth endpoints", "est": 4 }
                                ]
                            }
                        ]
                    },
                    "array_shorthand": [
                        {
                            "space": "Client",
                            "project": "website-redesign",
                            "activities": [{ "name": "Landing page", "est": 3 }]
                        }
                    ]
                }
            },
            {
                "command": "tomatui sessions list|edit|delete",
                "examples": [
                    "tomatui sessions list --space Work --json",
                    "tomatui sessions list --unassigned --json",
                    "tomatui sessions edit <ID_PREFIX> --project backend-api --activity \"API schema\" --notes \"Done\" --json"
                ]
            },
            {
                "command": "tomatui stats [--range today|week|month|year|all] [--space <SPACE>] [--project <PROJECT>] --json",
                "examples": [
                    "tomatui stats --range week --json",
                    "tomatui stats --range month --space Work --json"
                ]
            },
            {
                "command": "tomatui export [--format json|csv] [--from YYYY-MM-DD] [--to YYYY-MM-DD] [--space <SPACE>] [--project <PROJECT>]",
                "examples": [
                    "tomatui export --format csv --space Work"
                ]
            },
            {
                "command": "tomatui start [PROJECT] [--space <SPACE>] [--activity <ACT>] [--duration <M>] [--batches <N>] [--break <M>]",
                "shorthand": "tomatui PROJECT --25 --2",
                "description": "Launch interactive TUI timer"
            },
            {
                "command": "tomatui db-path --json",
                "description": "Print active SQLite database path"
            }
        ]
    })
}

pub fn agent_guide_markdown(db: &Database) -> String {
    format!(
        r#"# tomatui CLI & Agent Guide (v{version})

Data model: **Space** (optional: team, client, org) → **Project** → **Activity** → **Session**
Active SQLite DB: `{db_path}`
Tip: Pass `--json` to any management, import, stats, or query command for structured output.

## Spaces, Projects & Activities
- `tomatui spaces list --json`
- `tomatui spaces add <NAME> --json`
- `tomatui spaces delete <NAME> --json` (keeps projects ungrouped)
- `tomatui projects list --json`
- `tomatui projects add <NAME> [--space <SPACE>] [--color <HEX>] --json`
- `tomatui activities add <PROJECT> <NAME> [--est <N>] --json`
- `tomatui activities list [--project <PROJECT>] [--all] --json`
- `tomatui activities done <ID_PREFIX> --json`

## Bulk Import (Idempotent & Transactional)
Pipe JSON via stdin or pass a file path (`--dry-run` validates without writing):
```bash
cat <<'EOF' | tomatui import --json
{{
  "spaces": ["School", "Work", "Client", "Personal"],
  "projects": [
    {{
      "name": "backend-api",
      "space": "Work",
      "activities": ["API schema", {{ "name": "Auth endpoints", "est": 4 }}]
    }}
  ]
}}
EOF
```
*(Also accepts a top-level JSON array of project objects using `"name"` or `"project"`.)*

## Sessions, Stats & Export
- `tomatui sessions list [--space <SPACE>] [--project <PROJECT>] [--unassigned] --json`
- `tomatui sessions edit <ID_PREFIX> [--project <P>] [--activity <A>] [--status completed|partial|skipped] [--notes <TEXT>] --json`
- `tomatui stats [--range today|week|month|year|all] [--space <SPACE>] [--project <PROJECT>] --json`
- `tomatui export --format json|csv [--space <SPACE>] [--project <PROJECT>]`
- `tomatui guide --json` (machine-readable JSON version of this guide)
"#,
        version = env!("CARGO_PKG_VERSION"),
        db_path = db.path().display()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shorthand_project_duration_batches() {
        let args = normalize_shorthand_args(["tomatui", "backend-api", "--25", "--2"]);
        let cli = Cli::try_parse_from(args).unwrap();
        match cli.command {
            Some(Command::Start {
                project,
                duration,
                batches,
                break_mins,
                ..
            }) => {
                assert_eq!(project.as_deref(), Some("backend-api"));
                assert_eq!(duration, Some(25));
                assert_eq!(batches, Some(2));
                assert_eq!(break_mins, None);
            }
            other => panic!("unexpected command: {:?}", other),
        }
    }

    #[test]
    fn test_shorthand_unassigned_duration_only() {
        let args = normalize_shorthand_args(["tomatui", "--50", "--3", "--break", "10"]);
        let cli = Cli::try_parse_from(args).unwrap();
        match cli.command {
            Some(Command::Start {
                project,
                duration,
                batches,
                break_mins,
                ..
            }) => {
                assert_eq!(project, None);
                assert_eq!(duration, Some(50));
                assert_eq!(batches, Some(3));
                assert_eq!(break_mins, Some(10));
            }
            other => panic!("unexpected command: {:?}", other),
        }
    }

    #[test]
    fn test_standard_subcommand_passes_through() {
        let args = normalize_shorthand_args(["tomatui", "stats", "--range", "week", "--json"]);
        let cli = Cli::try_parse_from(args).unwrap();
        assert!(cli.json);
        assert!(matches!(cli.command, Some(Command::Stats { .. })));
    }

    #[test]
    fn test_import_spaces_and_guide_subcommands_parse() {
        for argv in [
            vec!["tomatui", "spaces", "add", "Work", "--json"],
            vec!["tomatui", "import", "--dry-run", "--json"],
            vec!["tomatui", "guide", "--json"],
            vec!["tomatui", "agent-help"],
            vec!["tomatui", "schema", "--json"],
        ] {
            let norm = normalize_shorthand_args(argv);
            let cli = Cli::try_parse_from(norm).unwrap();
            assert!(!matches!(cli.command, Some(Command::Start { .. })));
        }
    }
}
