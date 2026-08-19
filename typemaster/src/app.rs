//! Top-level application state, screen navigation, and input handling.
//!
//! Input handling ([`App::on_key`]) is separated from terminal I/O and takes an
//! explicit `now_ms` plus an optional [`Db`], so navigation, the typing flow,
//! and lesson progression can be unit-tested without a terminal, wall clock, or
//! database.

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use engine::curriculum::{self, Curriculum, Lesson, LessonProgress, Outcome};
use engine::metrics::CHARS_PER_WORD;
use engine::session::{Session, SessionState};

use crate::db::{DashboardStats, Db, ProgressRecord, SessionSummary};
use crate::themes::{Theme, THEMES};

/// Maximum data points shown in the progress chart / used for history.
const HISTORY_LIMIT: u32 = 60;
/// Number of rows shown on the leaderboard.
const LEADERBOARD_LIMIT: u32 = 10;
/// Fallback target text if a lesson fails to generate (e.g. missing corpus).
const FALLBACK_TEXT: &str = "the quick brown fox jumps over the lazy dog";
/// Frames the splash screen stays up before auto-advancing (~1.3s at 60fps).
const SPLASH_FRAMES: u64 = 80;

/// The top-level screens the user can navigate between.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    /// Animated launch splash.
    Splash,
    /// Home screen: stats, streak, and the main menu.
    Dashboard,
    /// Lesson browser: phases, lessons, lock/progress state.
    Curriculum,
    /// Active typing view.
    Lesson,
    /// Post-session results breakdown.
    Results,
    /// Per-key heatmap of the last session.
    Heatmap,
    /// Net-WPM history chart and leaderboard.
    Progress,
    /// Settings: theme and display toggles.
    Settings,
}

/// Dashboard menu entries, in display order.
pub const MENU_ITEMS: [&str; 4] = ["Lessons", "Progress", "Heatmap", "Settings"];

/// Outcome of handling a key, so the caller can react (persist, quit).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyOutcome {
    /// Nothing further to do.
    None,
    /// The session just finished and should be persisted.
    SessionFinished,
    /// The application should exit.
    Quit,
}

/// Current Unix-epoch time in milliseconds.
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Application state.
pub struct App {
    curriculum: Curriculum,
    progress: HashMap<String, LessonProgress>,
    current: usize,
    browse: usize,
    /// The lesson currently loaded (a curriculum lesson or a custom import).
    active: Lesson,
    custom_active: bool,
    session: Session,
    theme_idx: usize,
    screen: Screen,
    show_keyboard: bool,
    show_finger_guide: bool,
    show_ghost: bool,
    audio_enabled: bool,
    pending_bell: bool,
    /// Settings: first Ctrl+X arms reset; second confirms.
    reset_armed: bool,
    show_help: bool,
    menu_idx: usize,
    has_session: bool,
    last_outcome: Option<Outcome>,
    frame: u64,
    stats: DashboardStats,
    history: Vec<f64>,
    leaderboard: Vec<SessionSummary>,
}

impl App {
    /// Builds an app over the full curriculum, starting on the dashboard.
    pub fn new() -> engine::Result<Self> {
        let curriculum = Curriculum::new();
        let active = curriculum.lessons()[0].clone();
        let session = build_session(&active);
        Ok(App {
            curriculum,
            progress: HashMap::new(),
            current: 0,
            browse: 0,
            active,
            custom_active: false,
            session,
            theme_idx: 0,
            screen: Screen::Dashboard,
            show_keyboard: true,
            show_finger_guide: false,
            show_ghost: false,
            audio_enabled: false,
            pending_bell: false,
            reset_armed: false,
            show_help: false,
            menu_idx: 0,
            has_session: false,
            last_outcome: None,
            frame: 0,
            stats: DashboardStats::default(),
            history: Vec::new(),
            leaderboard: Vec::new(),
        })
    }

    /// Loads persisted progress and dashboard stats from `db` at startup.
    pub fn init(&mut self, db: Option<&Db>, now_ms: u64) {
        if let Some(db) = db {
            for rec in db.load_progress() {
                self.progress.insert(
                    rec.lesson_id,
                    LessonProgress {
                        consecutive_passes: rec.consecutive_passes,
                        best_net_wpm: rec.best_net_wpm,
                        completed: rec.completed,
                    },
                );
            }
        }
        self.refresh_dashboard(db, now_ms);
    }

    /// Shows the launch splash (called by the binary, not in tests).
    pub fn show_splash(&mut self) {
        self.screen = Screen::Splash;
    }

    /// Replaces the active lesson with imported custom text and starts it.
    pub fn load_custom(&mut self, text: String) {
        self.active = Lesson::custom(text);
        self.custom_active = true;
        self.session = build_session(&self.active);
        self.last_outcome = None;
        self.screen = Screen::Lesson;
    }

    // --- Accessors used by the screens -------------------------------------

    /// The active theme.
    pub fn theme(&self) -> &'static Theme {
        &THEMES[self.theme_idx]
    }
    /// The current screen.
    pub fn screen(&self) -> Screen {
        self.screen
    }
    /// The current session.
    pub fn session(&self) -> &Session {
        &self.session
    }
    /// The full curriculum.
    pub fn curriculum(&self) -> &Curriculum {
        &self.curriculum
    }
    /// Persisted progress, keyed by lesson id.
    pub fn progress(&self) -> &HashMap<String, LessonProgress> {
        &self.progress
    }
    /// Index of the lesson currently selected in the curriculum.
    pub fn current_index(&self) -> usize {
        self.current
    }
    /// Index highlighted in the curriculum browser.
    pub fn browse_index(&self) -> usize {
        self.browse
    }
    /// The lesson currently loaded.
    pub fn current_lesson(&self) -> &Lesson {
        &self.active
    }
    /// The result of the most recent finished session, if any.
    pub fn last_outcome(&self) -> Option<Outcome> {
        self.last_outcome
    }
    /// Whether the keyboard visualization is enabled.
    pub fn show_keyboard(&self) -> bool {
        self.show_keyboard
    }
    /// Whether the finger guide is enabled.
    pub fn show_finger_guide(&self) -> bool {
        self.show_finger_guide
    }
    /// Whether the ghost racer is enabled.
    pub fn show_ghost(&self) -> bool {
        self.show_ghost
    }
    /// Whether audio feedback is enabled.
    pub fn audio_enabled(&self) -> bool {
        self.audio_enabled
    }
    /// Whether the help overlay is shown.
    pub fn show_help(&self) -> bool {
        self.show_help
    }
    /// Selected dashboard menu index.
    pub fn menu_idx(&self) -> usize {
        self.menu_idx
    }
    /// Cached dashboard statistics.
    pub fn stats(&self) -> &DashboardStats {
        &self.stats
    }
    /// Cached net-WPM history (oldest→newest).
    pub fn history(&self) -> &[f64] {
        &self.history
    }
    /// Cached leaderboard rows.
    pub fn leaderboard(&self) -> &[SessionSummary] {
        &self.leaderboard
    }
    /// Whether a finished session exists to inspect (heatmap/results).
    pub fn has_session(&self) -> bool {
        self.has_session
    }

    /// Best net WPM recorded for the active lesson, if any (drives the ghost).
    pub fn best_wpm_current(&self) -> f64 {
        self.progress
            .get(&self.active.id)
            .map(|p| p.best_net_wpm)
            .unwrap_or(0.0)
    }

    /// The ghost cursor position at `now_ms`: how far a run at your best pace
    /// would have reached. `None` when the ghost is off or no best time exists.
    pub fn ghost_index(&self, now_ms: u64) -> Option<usize> {
        if !self.show_ghost {
            return None;
        }
        let best = self.best_wpm_current();
        if best <= 0.0 {
            return None;
        }
        let minutes = self.session.elapsed_ms(now_ms) as f64 / 60_000.0;
        let chars = (best * CHARS_PER_WORD * minutes).round() as usize;
        let last = self.session.total_chars().saturating_sub(1);
        Some(chars.min(last))
    }

    /// Whether a reset-progress confirmation is armed.
    pub fn reset_armed(&self) -> bool {
        self.reset_armed
    }

    /// Clears in-memory progress and queues a full DB wipe when `db` is set.
    pub fn reset_progress(&mut self, db: Option<&Db>, now_ms: u64) {
        self.progress.clear();
        self.has_session = false;
        self.last_outcome = None;
        self.history.clear();
        self.leaderboard.clear();
        self.stats = DashboardStats::default();
        self.reset_armed = false;
        if let Some(db) = db {
            db.reset_all();
        }
        self.refresh_dashboard(db, now_ms);
    }

    /// Takes the pending error-bell flag, if set (the caller emits the bell).
    pub fn take_bell(&mut self) -> bool {
        std::mem::take(&mut self.pending_bell)
    }

    /// Advances the frame counter; auto-dismisses the splash after a delay.
    pub fn tick(&mut self) {
        self.frame = self.frame.wrapping_add(1);
        if self.screen == Screen::Splash && self.frame > SPLASH_FRAMES {
            self.screen = Screen::Dashboard;
        }
    }

    /// Frames elapsed (used by the splash animation).
    pub fn frame(&self) -> u64 {
        self.frame
    }

    /// Whether the block cursor should be drawn this frame (≈2 Hz at 60fps).
    pub fn cursor_blink_on(&self) -> bool {
        (self.frame / 30).is_multiple_of(2)
    }

    /// The progress record for the active lesson, for persistence.
    pub fn current_progress_record(&self) -> ProgressRecord {
        let id = self.active.id.clone();
        let p = self.progress.get(&id).cloned().unwrap_or_default();
        ProgressRecord {
            lesson_id: id,
            consecutive_passes: p.consecutive_passes,
            best_net_wpm: p.best_net_wpm,
            completed: p.completed,
        }
    }

    /// Whether the just-finished session belongs to a tracked curriculum lesson.
    pub fn is_custom(&self) -> bool {
        self.custom_active
    }

    /// Starts a fresh session for the active lesson and shows the lesson screen.
    pub fn restart(&mut self) {
        self.session = build_session(&self.active);
        self.last_outcome = None;
        self.screen = Screen::Lesson;
    }

    /// Switches to curriculum lesson `index` (if valid) and starts it.
    fn start_lesson(&mut self, index: usize) {
        if let Some(lesson) = self.curriculum.get(index) {
            self.current = index;
            self.active = lesson.clone();
            self.custom_active = false;
            self.restart();
        }
    }

    /// Whether the current screen is accepting typing input.
    fn is_typing(&self) -> bool {
        self.screen == Screen::Lesson
            && matches!(
                self.session.state(),
                SessionState::Idle | SessionState::Active
            )
    }

    fn cycle_theme(&mut self) {
        self.theme_idx = (self.theme_idx + 1) % THEMES.len();
    }

    fn refresh_dashboard(&mut self, db: Option<&Db>, now_ms: u64) {
        if let Some(db) = db {
            self.stats = db.dashboard_stats(now_ms);
        }
    }

    fn refresh_progress_view(&mut self, db: Option<&Db>) {
        if let Some(db) = db {
            self.history = db.wpm_history(HISTORY_LIMIT);
            self.leaderboard = db.best_sessions(LEADERBOARD_LIMIT);
        }
    }

    fn activate_menu(&mut self, db: Option<&Db>) {
        match self.menu_idx {
            0 => {
                self.browse = self.current;
                self.screen = Screen::Curriculum;
            }
            1 => {
                self.refresh_progress_view(db);
                self.screen = Screen::Progress;
            }
            2 => self.screen = Screen::Heatmap,
            _ => self.screen = Screen::Settings,
        }
    }

    /// Moves to the next unlocked lesson, if any.
    fn go_next_lesson(&mut self) {
        if let Some(next) = self.curriculum.next_unlocked(self.current, &self.progress) {
            self.start_lesson(next);
        }
    }

    /// Moves to the previous lesson (always reachable from the current one).
    fn go_prev_lesson(&mut self) {
        if self.current > 0 {
            self.start_lesson(self.current - 1);
        }
    }

    /// Evaluates a just-finished session, updating in-memory progress.
    fn finish_session(&mut self, now_ms: u64) {
        let end = self.session.ended_at().unwrap_or(now_ms);
        let net_wpm = self.session.net_wpm(end);
        let accuracy = self.session.accuracy();
        if !self.custom_active {
            let outcome = curriculum::evaluate(&mut self.progress, &self.active, net_wpm, accuracy);
            self.last_outcome = Some(outcome);
        } else {
            self.last_outcome = None;
        }
        self.has_session = true;
        self.screen = Screen::Results;
    }

    /// Handles a key event at `now_ms`, optionally reading from `db` on screen
    /// transitions. Returns what the caller should do.
    pub fn on_key(&mut self, key: KeyEvent, now_ms: u64, db: Option<&Db>) -> KeyOutcome {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

        // Universal bindings, valid on every screen.
        match key.code {
            KeyCode::Char('c') if ctrl => return KeyOutcome::Quit,
            KeyCode::Char('t') if ctrl => {
                self.cycle_theme();
                return KeyOutcome::None;
            }
            KeyCode::Char('n') if ctrl => {
                self.go_next_lesson();
                return KeyOutcome::None;
            }
            KeyCode::Char('p') if ctrl => {
                self.go_prev_lesson();
                return KeyOutcome::None;
            }
            _ => {}
        }

        // Any key dismisses the splash.
        if self.screen == Screen::Splash {
            self.screen = Screen::Dashboard;
            return KeyOutcome::None;
        }

        // The help overlay swallows input until dismissed.
        if self.show_help {
            if matches!(
                key.code,
                KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q')
            ) {
                self.show_help = false;
            }
            return KeyOutcome::None;
        }

        if self.is_typing() {
            self.handle_typing_key(key, ctrl, now_ms, db)
        } else {
            self.handle_command_key(key, ctrl, now_ms, db)
        }
    }

    /// Input while actively typing: printable characters feed the session;
    /// control combinations remain available.
    fn handle_typing_key(
        &mut self,
        key: KeyEvent,
        ctrl: bool,
        now_ms: u64,
        db: Option<&Db>,
    ) -> KeyOutcome {
        match key.code {
            KeyCode::Esc => {
                self.refresh_dashboard(db, now_ms);
                self.screen = Screen::Dashboard;
                KeyOutcome::None
            }
            KeyCode::Char('r') if ctrl => {
                self.restart();
                KeyOutcome::None
            }
            KeyCode::Char('h') if ctrl => {
                self.screen = Screen::Heatmap;
                KeyOutcome::None
            }
            KeyCode::Char('s') if ctrl => {
                self.screen = Screen::Settings;
                KeyOutcome::None
            }
            KeyCode::Char('k') if ctrl => {
                self.show_keyboard = !self.show_keyboard;
                KeyOutcome::None
            }
            KeyCode::Char('f') if ctrl => {
                self.show_finger_guide = !self.show_finger_guide;
                KeyOutcome::None
            }
            KeyCode::Char('g') if ctrl => {
                self.show_ghost = !self.show_ghost;
                KeyOutcome::None
            }
            KeyCode::Char('m') if ctrl => {
                self.audio_enabled = !self.audio_enabled;
                KeyOutcome::None
            }
            KeyCode::Char(c) if !ctrl => {
                if self.session.state() == SessionState::Idle {
                    let _ = self.session.start(now_ms);
                }
                let was_active = self.session.state() == SessionState::Active;
                let correct = self.session.record(&c.to_string(), now_ms).unwrap_or(true);
                if !correct && self.audio_enabled {
                    self.pending_bell = true;
                }
                if was_active && self.session.state() == SessionState::Finished {
                    self.finish_session(now_ms);
                    return KeyOutcome::SessionFinished;
                }
                KeyOutcome::None
            }
            _ => KeyOutcome::None,
        }
    }

    /// Input on non-typing screens: keys act as commands.
    fn handle_command_key(
        &mut self,
        key: KeyEvent,
        ctrl: bool,
        now_ms: u64,
        db: Option<&Db>,
    ) -> KeyOutcome {
        match key.code {
            KeyCode::Char('?') => {
                self.show_help = true;
                KeyOutcome::None
            }
            KeyCode::Esc | KeyCode::Char('q') => {
                if self.screen == Screen::Dashboard {
                    KeyOutcome::Quit
                } else {
                    self.reset_armed = false;
                    self.refresh_dashboard(db, now_ms);
                    self.screen = Screen::Dashboard;
                    KeyOutcome::None
                }
            }
            KeyCode::Char('r') if ctrl => {
                self.restart();
                KeyOutcome::None
            }
            KeyCode::Char('h') if ctrl => {
                self.screen = Screen::Heatmap;
                KeyOutcome::None
            }
            KeyCode::Char('s') if ctrl => {
                self.screen = Screen::Settings;
                KeyOutcome::None
            }
            KeyCode::Char('k') => {
                self.show_keyboard = !self.show_keyboard;
                KeyOutcome::None
            }
            KeyCode::Char('f') => {
                self.show_finger_guide = !self.show_finger_guide;
                KeyOutcome::None
            }
            KeyCode::Char('g') => {
                self.show_ghost = !self.show_ghost;
                KeyOutcome::None
            }
            KeyCode::Char('m') => {
                self.audio_enabled = !self.audio_enabled;
                KeyOutcome::None
            }
            KeyCode::Char('x') if ctrl => {
                if self.screen == Screen::Settings {
                    if self.reset_armed {
                        self.reset_progress(db, now_ms);
                    } else {
                        self.reset_armed = true;
                    }
                }
                KeyOutcome::None
            }
            KeyCode::Down | KeyCode::Tab => {
                self.move_selection(1);
                KeyOutcome::None
            }
            KeyCode::Up => {
                self.move_selection(-1);
                KeyOutcome::None
            }
            KeyCode::Enter => {
                self.confirm(db, now_ms);
                KeyOutcome::None
            }
            _ => KeyOutcome::None,
        }
    }

    /// Moves the active selection (dashboard menu or curriculum browser).
    fn move_selection(&mut self, delta: isize) {
        match self.screen {
            Screen::Dashboard => {
                let n = MENU_ITEMS.len() as isize;
                self.menu_idx = (((self.menu_idx as isize + delta) % n + n) % n) as usize;
            }
            Screen::Curriculum => {
                let n = self.curriculum.len() as isize;
                if n > 0 {
                    self.browse = (((self.browse as isize + delta) % n + n) % n) as usize;
                }
            }
            _ => {}
        }
    }

    /// Confirms the current selection (Enter).
    fn confirm(&mut self, db: Option<&Db>, now_ms: u64) {
        match self.screen {
            Screen::Dashboard => self.activate_menu(db),
            Screen::Curriculum if self.curriculum.is_unlocked(self.browse, &self.progress) => {
                self.start_lesson(self.browse);
            }
            Screen::Results => {
                self.refresh_dashboard(db, now_ms);
                self.screen = Screen::Dashboard;
            }
            _ => {}
        }
    }
}

/// Builds a session for `lesson`, falling back to a fixed phrase if its text
/// cannot be generated.
fn build_session(lesson: &Lesson) -> Session {
    let text = lesson
        .generate_default()
        .ok()
        .filter(|t| !t.trim().is_empty())
        .unwrap_or_else(|| FALLBACK_TEXT.to_string());
    Session::new(&text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ch(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
    }
    fn ctrl(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
    }
    fn code(c: KeyCode) -> KeyEvent {
        KeyEvent::new(c, KeyModifiers::NONE)
    }

    fn open_curriculum(app: &mut App) {
        app.on_key(code(KeyCode::Enter), 0, None);
        assert_eq!(app.screen(), Screen::Curriculum);
    }

    fn type_out_session(app: &mut App) -> KeyOutcome {
        let target: String = app.session().target().concat();
        let mut now = 1_000;
        let mut outcome = KeyOutcome::None;
        for g in target.chars() {
            now += 50;
            outcome = app.on_key(ch(g), now, None);
        }
        outcome
    }

    #[test]
    fn starts_on_dashboard() {
        let app = App::new().unwrap();
        assert_eq!(app.screen(), Screen::Dashboard);
    }

    #[test]
    fn splash_dismisses_on_key_and_after_timeout() {
        let mut app = App::new().unwrap();
        app.show_splash();
        assert_eq!(app.screen(), Screen::Splash);
        app.on_key(code(KeyCode::Enter), 0, None);
        assert_eq!(app.screen(), Screen::Dashboard);

        let mut app2 = App::new().unwrap();
        app2.show_splash();
        for _ in 0..(SPLASH_FRAMES + 2) {
            app2.tick();
        }
        assert_eq!(app2.screen(), Screen::Dashboard);
    }

    #[test]
    fn lessons_menu_opens_curriculum_and_starts_first_lesson() {
        let mut app = App::new().unwrap();
        open_curriculum(&mut app);
        app.on_key(code(KeyCode::Enter), 0, None);
        assert_eq!(app.screen(), Screen::Lesson);
        assert_eq!(app.current_index(), 0);
    }

    #[test]
    fn locked_lesson_does_not_start() {
        let mut app = App::new().unwrap();
        open_curriculum(&mut app);
        app.on_key(code(KeyCode::Down), 0, None);
        assert_eq!(app.browse_index(), 1);
        app.on_key(code(KeyCode::Enter), 0, None);
        assert_eq!(app.screen(), Screen::Curriculum);
    }

    #[test]
    fn finishing_a_perfect_session_records_a_pass() {
        let mut app = App::new().unwrap();
        app.start_lesson(0);
        let outcome = type_out_session(&mut app);
        assert_eq!(outcome, KeyOutcome::SessionFinished);
        assert_eq!(app.screen(), Screen::Results);
        assert_eq!(app.last_outcome().unwrap().consecutive_passes, 1);
    }

    #[test]
    fn three_passes_complete_and_unlock_next() {
        let mut app = App::new().unwrap();
        app.start_lesson(0);
        for _ in 0..3 {
            type_out_session(&mut app);
            app.restart();
        }
        let first_id = app.curriculum().lessons()[0].id.clone();
        assert!(app.progress().get(&first_id).unwrap().completed);
        assert!(app.curriculum().is_unlocked(1, app.progress()));
    }

    #[test]
    fn ctrl_n_advances_after_unlock() {
        let mut app = App::new().unwrap();
        app.start_lesson(0);
        for _ in 0..3 {
            type_out_session(&mut app);
            app.restart();
        }
        app.on_key(ctrl('n'), 0, None);
        assert_eq!(app.current_index(), 1);
    }

    #[test]
    fn custom_text_runs_without_progress_tracking() {
        let mut app = App::new().unwrap();
        app.load_custom("hello world foo".to_string());
        assert_eq!(app.screen(), Screen::Lesson);
        assert!(app.is_custom());
        assert_eq!(app.session().target().concat(), "hello world foo");
        let outcome = type_out_session(&mut app);
        assert_eq!(outcome, KeyOutcome::SessionFinished);
        // Custom sessions don't record curriculum progress.
        assert!(app.last_outcome().is_none());
        assert!(app.progress().get("custom").is_none());
    }

    #[test]
    fn ghost_index_tracks_best_pace() {
        let mut app = App::new().unwrap();
        app.start_lesson(0);
        // No best yet, and ghost off → None.
        assert!(app.ghost_index(1_000).is_none());
        // Enable ghost and seed a best WPM for the active lesson.
        app.show_ghost = true;
        app.progress.insert(
            app.active.id.clone(),
            LessonProgress {
                consecutive_passes: 0,
                best_net_wpm: 60.0,
                completed: false,
            },
        );
        app.session.start(0).unwrap();
        // After 1s at 60 wpm: 60*5*(1/60) = 5 chars.
        assert_eq!(app.ghost_index(1_000), Some(5));
    }

    #[test]
    fn audio_toggle_and_error_bell() {
        let mut app = App::new().unwrap();
        app.on_key(ch('m'), 0, None); // enable audio (command mode)
        assert!(app.audio_enabled());
        app.start_lesson(0);
        // Ctrl+M also toggles while typing.
        app.on_key(ctrl('m'), 0, None);
        assert!(!app.audio_enabled());
        app.on_key(ctrl('m'), 0, None);
        assert!(app.audio_enabled());
        // Type a guaranteed-wrong key to trigger the bell; cursor stays put.
        let first = app.session().target()[0].chars().next().unwrap();
        let wrong = if first == 'z' { 'a' } else { 'z' };
        app.on_key(ch(wrong), 100, None);
        assert_eq!(app.session().cursor(), 0);
        assert!(app.take_bell());
        assert!(!app.take_bell()); // taken once
    }

    #[test]
    fn reset_progress_requires_double_ctrl_x() {
        let mut app = App::new().unwrap();
        app.progress.insert(
            "1.1".into(),
            LessonProgress {
                consecutive_passes: 2,
                best_net_wpm: 40.0,
                completed: false,
            },
        );
        app.screen = Screen::Settings;
        app.on_key(ctrl('x'), 0, None);
        assert!(app.reset_armed());
        assert!(!app.progress().is_empty());
        app.on_key(ctrl('x'), 0, None);
        assert!(!app.reset_armed());
        assert!(app.progress().is_empty());
    }

    #[test]
    fn theme_cycles_with_ctrl_t() {
        let mut app = App::new().unwrap();
        assert_eq!(app.theme().name, "void");
        app.on_key(ctrl('t'), 0, None);
        assert_eq!(app.theme().name, "light");
    }

    #[test]
    fn help_overlay_toggles_and_swallows_input() {
        let mut app = App::new().unwrap();
        app.on_key(ch('?'), 0, None);
        assert!(app.show_help());
        app.on_key(code(KeyCode::Tab), 0, None);
        assert!(app.show_help());
        assert_eq!(app.menu_idx(), 0);
        app.on_key(code(KeyCode::Esc), 0, None);
        assert!(!app.show_help());
    }
}
