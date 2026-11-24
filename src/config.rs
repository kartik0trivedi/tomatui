use crate::theme::ThemeChoice;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub work_duration_mins: u32,
    pub short_break_mins: u32,
    pub long_break_mins: u32,
    pub long_break_interval: u32,
    pub auto_start_breaks: bool,
    pub auto_start_work: bool,
    pub sound_enabled: bool,
    pub desktop_notifications: bool,
    pub transparent_background: bool,
    pub theme: ThemeChoice,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            work_duration_mins: 25,
            short_break_mins: 5,
            long_break_mins: 15,
            long_break_interval: 4,
            auto_start_breaks: true,
            auto_start_work: false,
            sound_enabled: true,
            desktop_notifications: true,
            transparent_background: true,
            theme: ThemeChoice::default(),
        }
    }
}

impl Config {
    pub fn sanitize(&mut self) {
        self.work_duration_mins = self.work_duration_mins.clamp(1, 120);
        self.short_break_mins = self.short_break_mins.clamp(1, 60);
        self.long_break_mins = self.long_break_mins.clamp(1, 90);
        self.long_break_interval = self.long_break_interval.clamp(1, 24);
    }
}
