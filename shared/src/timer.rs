use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TimerPhase {
    Work,
    ShortBreak,
    LongBreak,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimerSettings {
    pub work_minutes: u32,
    pub short_break_minutes: u32,
    pub long_break_minutes: u32,
    pub sessions_before_long_break: u32,
}

impl Default for TimerSettings {
    fn default() -> Self {
        TimerSettings {
            work_minutes: 25,
            short_break_minutes: 5,
            long_break_minutes: 15,
            sessions_before_long_break: 4,
        }
    }
}

impl TimerSettings {
    pub fn duration_secs(&self, phase: TimerPhase) -> u32 {
        let minutes = match phase {
            TimerPhase::Work => self.work_minutes,
            TimerPhase::ShortBreak => self.short_break_minutes,
            TimerPhase::LongBreak => self.long_break_minutes,
        };
        minutes * 60
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimerState {
    pub phase: TimerPhase,
    pub remaining_secs: u32,
    pub is_running: bool,
    pub completed_work_sessions: u32,
    pub settings: TimerSettings,
}

impl TimerState {
    pub fn new(settings: TimerSettings) -> Self {
        TimerState {
            remaining_secs: settings.duration_secs(TimerPhase::Work),
            phase: TimerPhase::Work,
            is_running: false,
            completed_work_sessions: 0,
            settings,
        }
    }

    /// 現フェーズを終え、次のフェーズへ進める。作業セッションを既定回数
    /// （デフォルト4回）終えるごとに長い休憩を挟み、休憩の後は必ず作業に戻る。
    pub fn advance_phase(&mut self) {
        self.phase = match self.phase {
            TimerPhase::Work => {
                self.completed_work_sessions += 1;
                if self
                    .completed_work_sessions
                    .is_multiple_of(self.settings.sessions_before_long_break)
                {
                    TimerPhase::LongBreak
                } else {
                    TimerPhase::ShortBreak
                }
            }
            TimerPhase::ShortBreak | TimerPhase::LongBreak => TimerPhase::Work,
        };
        self.remaining_secs = self.settings.duration_secs(self.phase);
        self.is_running = false;
    }

    pub fn reset_current_phase(&mut self) {
        self.remaining_secs = self.settings.duration_secs(self.phase);
        self.is_running = false;
    }
}

/// 残り秒数を`mm:ss`形式にする。フロントのリング表示・Phase 5のトレイタイトル表示の両方で使う。
pub fn format_mm_ss(total_seconds: u32) -> String {
    format!("{:02}:{:02}", total_seconds / 60, total_seconds % 60)
}

/// フェーズの日本語ラベル(フロント表示専用)。
pub fn phase_label(phase: TimerPhase) -> &'static str {
    match phase {
        TimerPhase::Work => "作業",
        TimerPhase::ShortBreak => "短い休憩",
        TimerPhase::LongBreak => "長い休憩",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> TimerSettings {
        TimerSettings::default()
    }

    #[test]
    fn work_session_moves_to_short_break_by_default() {
        let mut state = TimerState::new(settings());
        state.advance_phase();
        assert_eq!(state.phase, TimerPhase::ShortBreak);
        assert_eq!(state.completed_work_sessions, 1);
    }

    #[test]
    fn fourth_work_session_moves_to_long_break() {
        let mut state = TimerState::new(settings());
        for _ in 0..3 {
            state.advance_phase(); // Work -> ShortBreak
            state.advance_phase(); // ShortBreak -> Work
        }
        state.advance_phase(); // 4回目のWork -> LongBreak
        assert_eq!(state.phase, TimerPhase::LongBreak);
        assert_eq!(state.completed_work_sessions, 4);
    }

    #[test]
    fn break_always_returns_to_work() {
        let mut state = TimerState::new(settings());
        state.advance_phase(); // -> ShortBreak
        state.advance_phase(); // -> Work
        assert_eq!(state.phase, TimerPhase::Work);
    }

    #[test]
    fn reset_current_phase_restores_full_duration_and_stops() {
        let mut state = TimerState::new(settings());
        state.remaining_secs = 10;
        state.is_running = true;
        state.reset_current_phase();
        assert_eq!(state.remaining_secs, 25 * 60);
        assert!(!state.is_running);
    }

    #[test]
    fn format_mm_ss_pads_with_zero() {
        assert_eq!(format_mm_ss(65), "01:05");
        assert_eq!(format_mm_ss(600), "10:00");
    }
}
