use crate::config::Config;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PomodoroPhase {
    Work,
    ShortBreak,
    LongBreak,
}

impl PomodoroPhase {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Work => "work",
            Self::ShortBreak => "short_break",
            Self::LongBreak => "long_break",
        }
    }

    pub fn from_db(s: &str) -> Self {
        match s {
            "short_break" => Self::ShortBreak,
            "long_break" => Self::LongBreak,
            _ => Self::Work,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Work => "Focusing",
            Self::ShortBreak => "Short Break",
            Self::LongBreak => "Long Break",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerStatus {
    Idle,
    Running,
    Paused,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionPlan {
    pub work_mins: u32,
    pub break_mins: u32,
    pub batches: u32,
}

impl SessionPlan {
    pub fn new(work_mins: u32, break_mins: u32, batches: u32) -> Self {
        Self {
            work_mins: work_mins.clamp(1, 120),
            break_mins: break_mins.clamp(1, 60),
            batches: batches.clamp(1, 24),
        }
    }

    pub fn label(&self) -> String {
        if self.batches > 1 {
            format!("{}m × {}  ·  {}m break", self.work_mins, self.batches, self.break_mins)
        } else {
            format!("{}m × 1", self.work_mins)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimerEvent {
    PhaseCompleted {
        finished_phase: PomodoroPhase,
        next_phase: PomodoroPhase,
        actual_secs: u32,
        batch_finished: bool,
    },
}

#[derive(Debug, Clone)]
pub struct PomodoroTimer {
    pub phase: PomodoroPhase,
    pub status: TimerStatus,
    pub plan: SessionPlan,
    pub current_batch: u32,
    pub time_remaining_secs: u32,
    pub total_duration_secs: u32,
    pub elapsed_secs: u32,
}

impl PomodoroTimer {
    pub fn new(config: &Config) -> Self {
        let plan = SessionPlan::new(config.work_duration_mins, config.short_break_mins, 1);
        Self::with_plan(plan)
    }

    pub fn with_plan(plan: SessionPlan) -> Self {
        let total = plan.work_mins.saturating_mul(60);
        Self {
            phase: PomodoroPhase::Work,
            status: TimerStatus::Idle,
            plan,
            current_batch: 1,
            time_remaining_secs: total,
            total_duration_secs: total,
            elapsed_secs: 0,
        }
    }

    pub fn set_plan(&mut self, plan: SessionPlan) {
        self.plan = plan;
        if self.status == TimerStatus::Idle {
            self.phase = PomodoroPhase::Work;
            self.current_batch = 1;
            let total = plan.work_mins.saturating_mul(60);
            self.total_duration_secs = total;
            self.time_remaining_secs = total;
            self.elapsed_secs = 0;
        }
    }

    pub fn start(&mut self) {
        if self.status == TimerStatus::Idle {
            self.phase = PomodoroPhase::Work;
            self.current_batch = 1;
            let total = self.plan.work_mins.saturating_mul(60);
            self.total_duration_secs = total;
            self.time_remaining_secs = total;
            self.elapsed_secs = 0;
        }
        self.status = TimerStatus::Running;
    }

    pub fn toggle_pause(&mut self) {
        match self.status {
            TimerStatus::Running => self.status = TimerStatus::Paused,
            TimerStatus::Paused => self.status = TimerStatus::Running,
            TimerStatus::Idle => self.start(),
        }
    }

    pub fn stop_to_idle(&mut self) -> u32 {
        let spent = self.elapsed_secs;
        self.status = TimerStatus::Idle;
        self.phase = PomodoroPhase::Work;
        self.current_batch = 1;
        let total = self.plan.work_mins.saturating_mul(60);
        self.total_duration_secs = total;
        self.time_remaining_secs = total;
        self.elapsed_secs = 0;
        spent
    }

    pub fn progress_ratio(&self) -> f64 {
        if self.total_duration_secs == 0 {
            return 0.0;
        }
        (self.elapsed_secs as f64 / self.total_duration_secs as f64).clamp(0.0, 1.0)
    }

    pub fn formatted_time(&self) -> (u32, u32) {
        (self.time_remaining_secs / 60, self.time_remaining_secs % 60)
    }

    pub fn tick(&mut self, config: &Config) -> Option<TimerEvent> {
        if self.status != TimerStatus::Running {
            return None;
        }

        if self.time_remaining_secs > 0 {
            self.time_remaining_secs -= 1;
            self.elapsed_secs = self.elapsed_secs.saturating_add(1);
        }

        if self.time_remaining_secs == 0 {
            let finished_phase = self.phase;
            let actual_secs = self.elapsed_secs;
            let (next_phase, batch_finished) = self.advance_after_completion(config);
            Some(TimerEvent::PhaseCompleted {
                finished_phase,
                next_phase,
                actual_secs,
                batch_finished,
            })
        } else {
            None
        }
    }

    fn advance_after_completion(&mut self, config: &Config) -> (PomodoroPhase, bool) {
        match self.phase {
            PomodoroPhase::Work => {
                if self.current_batch >= self.plan.batches {
                    // Entire planned batch is complete! Return to Idle ready for next run.
                    self.status = TimerStatus::Idle;
                    self.phase = PomodoroPhase::Work;
                    self.current_batch = 1;
                    let total = self.plan.work_mins.saturating_mul(60);
                    self.total_duration_secs = total;
                    self.time_remaining_secs = total;
                    self.elapsed_secs = 0;
                    (PomodoroPhase::Work, true)
                } else {
                    let next = if self.current_batch % config.long_break_interval == 0
                        && self.plan.batches > config.long_break_interval
                    {
                        PomodoroPhase::LongBreak
                    } else {
                        PomodoroPhase::ShortBreak
                    };
                    let break_mins = match next {
                        PomodoroPhase::LongBreak => config.long_break_mins,
                        _ => self.plan.break_mins,
                    };
                    self.phase = next;
                    self.total_duration_secs = break_mins.saturating_mul(60);
                    self.time_remaining_secs = self.total_duration_secs;
                    self.elapsed_secs = 0;
                    self.status = if config.auto_start_breaks {
                        TimerStatus::Running
                    } else {
                        TimerStatus::Paused
                    };
                    (next, false)
                }
            }
            PomodoroPhase::ShortBreak | PomodoroPhase::LongBreak => {
                self.current_batch = (self.current_batch + 1).min(self.plan.batches);
                self.phase = PomodoroPhase::Work;
                self.total_duration_secs = self.plan.work_mins.saturating_mul(60);
                self.time_remaining_secs = self.total_duration_secs;
                self.elapsed_secs = 0;
                self.status = if config.auto_start_work || self.plan.batches > 1 {
                    TimerStatus::Running
                } else {
                    TimerStatus::Paused
                };
                (PomodoroPhase::Work, false)
            }
        }
    }

    pub fn skip(&mut self, config: &Config) -> (PomodoroPhase, u32, bool) {
        let skipped_phase = self.phase;
        let spent = self.elapsed_secs;
        let (_, batch_finished) = self.advance_after_completion(config);
        (skipped_phase, spent, batch_finished)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_two_batch_plan_transitions() {
        let cfg = Config {
            auto_start_breaks: true,
            auto_start_work: true,
            ..Default::default()
        };
        let mut timer = PomodoroTimer::with_plan(SessionPlan::new(25, 5, 2));
        timer.start();
        assert_eq!(timer.status, TimerStatus::Running);
        assert_eq!(timer.current_batch, 1);

        // Fast-forward to last second of Work #1
        timer.time_remaining_secs = 1;
        timer.elapsed_secs = 1499;
        let ev = timer.tick(&cfg).unwrap();
        assert_eq!(
            ev,
            TimerEvent::PhaseCompleted {
                finished_phase: PomodoroPhase::Work,
                next_phase: PomodoroPhase::ShortBreak,
                actual_secs: 1500,
                batch_finished: false,
            }
        );
        assert_eq!(timer.phase, PomodoroPhase::ShortBreak);
        assert_eq!(timer.time_remaining_secs, 300);

        // Finish Break
        timer.time_remaining_secs = 1;
        timer.elapsed_secs = 299;
        let ev2 = timer.tick(&cfg).unwrap();
        assert_eq!(
            ev2,
            TimerEvent::PhaseCompleted {
                finished_phase: PomodoroPhase::ShortBreak,
                next_phase: PomodoroPhase::Work,
                actual_secs: 300,
                batch_finished: false,
            }
        );
        assert_eq!(timer.phase, PomodoroPhase::Work);
        assert_eq!(timer.current_batch, 2);

        // Finish Work #2 -> Batch finished -> Idle
        timer.time_remaining_secs = 1;
        timer.elapsed_secs = 1499;
        let ev3 = timer.tick(&cfg).unwrap();
        assert_eq!(
            ev3,
            TimerEvent::PhaseCompleted {
                finished_phase: PomodoroPhase::Work,
                next_phase: PomodoroPhase::Work,
                actual_secs: 1500,
                batch_finished: true,
            }
        );
        assert_eq!(timer.status, TimerStatus::Idle);
    }
}
