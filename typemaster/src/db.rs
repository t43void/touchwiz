//! SQLite persistence (specification Section 7).
//!
//! All writes happen on a dedicated background task fed by an unbounded channel,
//! so the render thread never blocks on the database (Quality rule 6). The
//! database connection lives entirely inside that task.

use std::path::Path;

use engine::adaptive::{target_latency_ms, Card};
use engine::curriculum::Lesson;
use engine::heatmap::Heatmap;
use engine::session::Session;
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio::task::JoinHandle;

/// A per-key statistic row destined for `key_stats`.
#[derive(Debug, Clone)]
pub struct KeyStatRecord {
    /// The key character.
    pub key_char: String,
    /// Average latency in milliseconds.
    pub avg_latency_ms: f64,
    /// Error rate in `[0, 1]`.
    pub error_rate: f64,
    /// Total attempts.
    pub total_hits: u32,
}

/// A per-bigram statistic row destined for `bigram_stats`.
#[derive(Debug, Clone)]
pub struct BigramStatRecord {
    /// The two-character bigram.
    pub bigram: String,
    /// Average latency in milliseconds.
    pub avg_latency_ms: f64,
    /// Error rate in `[0, 1]`.
    pub error_rate: f64,
    /// Total attempts.
    pub total_hits: u32,
}

/// A per-word review row used to update `word_mastery`.
#[derive(Debug, Clone)]
pub struct WordStatRecord {
    /// The intended word.
    pub word: String,
    /// Error rate in `[0, 1]`.
    pub error_rate: f32,
    /// Average inter-key latency in milliseconds.
    pub avg_latency_ms: f32,
}

/// A complete finished-session snapshot ready to persist.
#[derive(Debug, Clone)]
pub struct SessionRecord {
    /// Session start, Unix-epoch milliseconds.
    pub started_at: u64,
    /// Session end, Unix-epoch milliseconds.
    pub ended_at: u64,
    /// Lesson identifier.
    pub lesson_id: String,
    /// Phase number in `[1, 4]`.
    pub phase: u8,
    /// Final net WPM.
    pub net_wpm: f64,
    /// Final raw WPM.
    pub raw_wpm: f64,
    /// Final accuracy percentage.
    pub accuracy: f64,
    /// Final consistency score.
    pub consistency: f64,
    /// Total characters in the target.
    pub total_chars: u32,
    /// Total errors.
    pub error_count: u32,
    /// Duration in seconds.
    pub duration_secs: u32,
    /// Corpus identifier.
    pub corpus_type: String,
    /// Per-key statistics.
    pub key_stats: Vec<KeyStatRecord>,
    /// Per-bigram statistics.
    pub bigram_stats: Vec<BigramStatRecord>,
    /// Per-word review data.
    pub word_stats: Vec<WordStatRecord>,
}

impl SessionRecord {
    /// Builds a record from a finished `session` and its `lesson`.
    pub fn from_session(session: &Session, lesson: &Lesson) -> Self {
        let end = session
            .ended_at()
            .unwrap_or_else(|| session.started_at().unwrap_or(0));
        let hm = Heatmap::from_session(session);

        let key_stats = hm
            .keys()
            .iter()
            .map(|(&c, s)| KeyStatRecord {
                key_char: c.to_string(),
                avg_latency_ms: s.avg_latency_ms(),
                error_rate: s.error_rate(),
                total_hits: s.hits,
            })
            .collect();

        let bigram_stats = hm
            .bigrams()
            .iter()
            .map(|(b, s)| BigramStatRecord {
                bigram: b.clone(),
                avg_latency_ms: s.avg_latency_ms(),
                error_rate: s.error_rate(),
                total_hits: s.hits,
            })
            .collect();

        let word_stats = session
            .word_stats()
            .into_iter()
            .map(|w| WordStatRecord {
                word: w.word,
                error_rate: w.error_rate,
                avg_latency_ms: w.avg_latency_ms,
            })
            .collect();

        SessionRecord {
            started_at: session.started_at().unwrap_or(0),
            ended_at: end,
            lesson_id: lesson.id.clone(),
            phase: lesson.phase.number(),
            net_wpm: session.net_wpm(end),
            raw_wpm: session.raw_wpm(end),
            accuracy: session.accuracy(),
            consistency: session.consistency(),
            total_chars: session.total_chars() as u32,
            error_count: session.error_count(),
            duration_secs: (session.elapsed_ms(end) / 1000) as u32,
            corpus_type: lesson.content.corpus_type().to_string(),
            key_stats,
            bigram_stats,
            word_stats,
        }
    }
}

/// Aggregate statistics shown on the dashboard.
#[derive(Debug, Clone, Default)]
pub struct DashboardStats {
    /// Total sessions ever completed.
    pub total_sessions: u32,
    /// All-time best net WPM.
    pub best_net_wpm: f64,
    /// All-time best accuracy percentage.
    pub best_accuracy: f64,
    /// Net WPM of the most recent session.
    pub last_net_wpm: f64,
    /// Sessions completed today (UTC).
    pub today_sessions: u32,
    /// Minutes practiced today (UTC).
    pub today_minutes: u32,
}

/// A compact session summary for leaderboard views.
#[derive(Debug, Clone)]
pub struct SessionSummary {
    /// Net WPM.
    pub net_wpm: f64,
    /// Accuracy percentage.
    pub accuracy: f64,
}

/// A lesson-progress row for `lesson_progress`.
#[derive(Debug, Clone)]
pub struct ProgressRecord {
    /// Lesson identifier.
    pub lesson_id: String,
    /// Consecutive passing sessions.
    pub consecutive_passes: u32,
    /// Best net WPM on the lesson.
    pub best_net_wpm: f64,
    /// Whether the lesson is completed.
    pub completed: bool,
}

/// A write command processed by the background writer task.
enum Command {
    /// Persist a finished session and its derived statistics.
    Session(Box<SessionRecord>),
    /// Upsert a lesson's progress.
    Progress(ProgressRecord),
}

/// Handle to the background database writer plus a read connection.
pub struct Db {
    tx: UnboundedSender<Command>,
    handle: JoinHandle<()>,
    /// Connection used for synchronous reads on the main thread. Reads are only
    /// issued on screen transitions (never per-frame), so they do not block the
    /// render loop in practice.
    read: Connection,
}

impl Db {
    /// Opens (creating if needed) the database at `path`, runs migrations, and
    /// spawns the background writer task.
    pub fn open(path: &Path) -> rusqlite::Result<Db> {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let mut conn = Connection::open(path)?;
        // WAL lets the read connection proceed without blocking the writer.
        let _ = conn.pragma_update(None, "journal_mode", "WAL");
        migrate(&conn)?;

        let read = Connection::open(path)?;

        let (tx, mut rx) = unbounded_channel::<Command>();
        let handle = tokio::spawn(async move {
            while let Some(cmd) = rx.recv().await {
                let result = match cmd {
                    Command::Session(rec) => save_record(&mut conn, &rec),
                    Command::Progress(rec) => save_progress(&conn, &rec),
                };
                if let Err(e) = result {
                    // The render thread is gone by design; surface to stderr.
                    eprintln!("touchwiz: failed to persist: {e}");
                }
            }
        });
        Ok(Db { tx, handle, read })
    }

    /// Queues a finished session for persistence. Non-blocking.
    pub fn save(&self, record: SessionRecord) {
        let _ = self.tx.send(Command::Session(Box::new(record)));
    }

    /// Queues a lesson-progress upsert. Non-blocking.
    pub fn save_progress(&self, record: ProgressRecord) {
        let _ = self.tx.send(Command::Progress(record));
    }

    /// Loads all stored lesson progress as `(lesson_id, (passes, best, completed))`.
    pub fn load_progress(&self) -> Vec<ProgressRecord> {
        self.try_load_progress().unwrap_or_default()
    }

    fn try_load_progress(&self) -> rusqlite::Result<Vec<ProgressRecord>> {
        let mut stmt = self.read.prepare(
            "SELECT lesson_id, consecutive_passes, best_net_wpm, completed FROM lesson_progress",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(ProgressRecord {
                lesson_id: r.get(0)?,
                consecutive_passes: r.get::<_, i64>(1)? as u32,
                best_net_wpm: r.get(2)?,
                completed: r.get::<_, i64>(3)? != 0,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// Flushes pending writes and shuts down the writer task.
    pub async fn close(self) {
        drop(self.tx);
        let _ = self.handle.await;
    }

    /// Loads aggregate dashboard statistics as of `now_ms`. Returns defaults on
    /// any read error so the UI degrades gracefully.
    pub fn dashboard_stats(&self, now_ms: u64) -> DashboardStats {
        self.try_dashboard_stats(now_ms).unwrap_or_default()
    }

    fn try_dashboard_stats(&self, now_ms: u64) -> rusqlite::Result<DashboardStats> {
        let total_sessions: u32 =
            self.read
                .query_row("SELECT COUNT(*) FROM sessions", [], |r| r.get(0))?;
        let best_net_wpm: f64 = self
            .read
            .query_row(
                "SELECT value FROM personal_bests WHERE metric='net_wpm' ORDER BY value DESC LIMIT 1",
                [],
                |r| r.get(0),
            )
            .optional()?
            .unwrap_or(0.0);
        let best_accuracy: f64 = self
            .read
            .query_row(
                "SELECT value FROM personal_bests WHERE metric='accuracy' ORDER BY value DESC LIMIT 1",
                [],
                |r| r.get(0),
            )
            .optional()?
            .unwrap_or(0.0);
        let last_net_wpm: f64 = self
            .read
            .query_row(
                "SELECT net_wpm FROM sessions ORDER BY ended_at DESC LIMIT 1",
                [],
                |r| r.get(0),
            )
            .optional()?
            .unwrap_or(0.0);

        let today = utc_date(now_ms);
        let (today_sessions, today_minutes): (u32, u32) = self
            .read
            .query_row(
                "SELECT sessions_count, total_minutes FROM streaks WHERE date=?1",
                params![today],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?
            .unwrap_or((0, 0));

        Ok(DashboardStats {
            total_sessions,
            best_net_wpm,
            best_accuracy,
            last_net_wpm,
            today_sessions,
            today_minutes,
        })
    }

    /// Net-WPM history (oldest→newest) for the progress chart, up to `limit`.
    pub fn wpm_history(&self, limit: u32) -> Vec<f64> {
        self.try_wpm_history(limit).unwrap_or_default()
    }

    fn try_wpm_history(&self, limit: u32) -> rusqlite::Result<Vec<f64>> {
        let mut stmt = self.read.prepare(
            "SELECT net_wpm FROM (SELECT net_wpm, ended_at FROM sessions
             ORDER BY ended_at DESC LIMIT ?1) ORDER BY ended_at ASC",
        )?;
        let rows = stmt.query_map(params![limit], |r| r.get::<_, f64>(0))?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// The top sessions by net WPM (leaderboard), up to `limit`.
    pub fn best_sessions(&self, limit: u32) -> Vec<SessionSummary> {
        self.try_best_sessions(limit).unwrap_or_default()
    }

    fn try_best_sessions(&self, limit: u32) -> rusqlite::Result<Vec<SessionSummary>> {
        let mut stmt = self
            .read
            .prepare("SELECT net_wpm, accuracy FROM sessions ORDER BY net_wpm DESC LIMIT ?1")?;
        let rows = stmt.query_map(params![limit], |r| {
            Ok(SessionSummary {
                net_wpm: r.get(0)?,
                accuracy: r.get(1)?,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }
}

/// Creates all tables if they do not already exist.
fn migrate(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS sessions (
            id            INTEGER PRIMARY KEY AUTOINCREMENT,
            started_at    INTEGER NOT NULL,
            ended_at      INTEGER NOT NULL,
            lesson_id     TEXT NOT NULL,
            phase         INTEGER NOT NULL,
            net_wpm       REAL NOT NULL,
            raw_wpm       REAL NOT NULL,
            accuracy      REAL NOT NULL,
            consistency   REAL NOT NULL,
            total_chars   INTEGER NOT NULL,
            error_count   INTEGER NOT NULL,
            duration_secs INTEGER NOT NULL,
            corpus_type   TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS key_stats (
            id             INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id     INTEGER REFERENCES sessions(id),
            key_char       TEXT NOT NULL,
            avg_latency_ms REAL NOT NULL,
            error_rate     REAL NOT NULL,
            total_hits     INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS bigram_stats (
            id             INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id     INTEGER REFERENCES sessions(id),
            bigram         TEXT NOT NULL,
            avg_latency_ms REAL NOT NULL,
            error_rate     REAL NOT NULL,
            total_hits     INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS word_mastery (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            word            TEXT UNIQUE NOT NULL,
            easiness_factor REAL NOT NULL DEFAULT 2.5,
            interval_days   INTEGER NOT NULL DEFAULT 1,
            repetitions     INTEGER NOT NULL DEFAULT 0,
            avg_latency_ms  REAL NOT NULL DEFAULT 999.0,
            error_rate      REAL NOT NULL DEFAULT 1.0,
            next_review     INTEGER NOT NULL,
            last_seen       INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS personal_bests (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            metric      TEXT NOT NULL,
            value       REAL NOT NULL,
            session_id  INTEGER REFERENCES sessions(id),
            achieved_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS streaks (
            id             INTEGER PRIMARY KEY AUTOINCREMENT,
            date           TEXT UNIQUE NOT NULL,
            sessions_count INTEGER NOT NULL DEFAULT 1,
            total_minutes  INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS lesson_progress (
            id                 INTEGER PRIMARY KEY AUTOINCREMENT,
            lesson_id          TEXT UNIQUE NOT NULL,
            consecutive_passes INTEGER NOT NULL DEFAULT 0,
            best_net_wpm       REAL NOT NULL DEFAULT 0.0,
            completed          INTEGER NOT NULL DEFAULT 0
        );
        ",
    )
}

/// Upserts a single lesson's progress row.
fn save_progress(conn: &Connection, rec: &ProgressRecord) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO lesson_progress (lesson_id, consecutive_passes, best_net_wpm, completed)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(lesson_id) DO UPDATE SET
            consecutive_passes = excluded.consecutive_passes,
            best_net_wpm       = excluded.best_net_wpm,
            completed          = excluded.completed",
        params![
            rec.lesson_id,
            rec.consecutive_passes,
            rec.best_net_wpm,
            i64::from(rec.completed),
        ],
    )?;
    Ok(())
}

/// Persists one session and all of its derived statistics in a single
/// transaction, including personal-best and streak updates.
fn save_record(conn: &mut Connection, rec: &SessionRecord) -> rusqlite::Result<()> {
    let tx = conn.transaction()?;

    tx.execute(
        "INSERT INTO sessions (started_at, ended_at, lesson_id, phase, net_wpm, raw_wpm,
            accuracy, consistency, total_chars, error_count, duration_secs, corpus_type)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            rec.started_at as i64,
            rec.ended_at as i64,
            rec.lesson_id,
            rec.phase,
            rec.net_wpm,
            rec.raw_wpm,
            rec.accuracy,
            rec.consistency,
            rec.total_chars,
            rec.error_count,
            rec.duration_secs,
            rec.corpus_type,
        ],
    )?;
    let session_id = tx.last_insert_rowid();

    for k in &rec.key_stats {
        tx.execute(
            "INSERT INTO key_stats (session_id, key_char, avg_latency_ms, error_rate, total_hits)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                session_id,
                k.key_char,
                k.avg_latency_ms,
                k.error_rate,
                k.total_hits
            ],
        )?;
    }
    for b in &rec.bigram_stats {
        tx.execute(
            "INSERT INTO bigram_stats (session_id, bigram, avg_latency_ms, error_rate, total_hits)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                session_id,
                b.bigram,
                b.avg_latency_ms,
                b.error_rate,
                b.total_hits
            ],
        )?;
    }

    for w in &rec.word_stats {
        upsert_word(&tx, w, rec.phase, rec.ended_at)?;
    }

    update_personal_best(&tx, "net_wpm", rec.net_wpm, session_id, rec.ended_at)?;
    update_personal_best(&tx, "raw_wpm", rec.raw_wpm, session_id, rec.ended_at)?;
    update_personal_best(&tx, "accuracy", rec.accuracy, session_id, rec.ended_at)?;
    update_personal_best(
        &tx,
        "consistency",
        rec.consistency,
        session_id,
        rec.ended_at,
    )?;

    update_streak(&tx, rec.ended_at, rec.duration_secs)?;

    tx.commit()
}

/// Updates a word's spaced-repetition card via SM-2, inserting or updating its
/// `word_mastery` row.
fn upsert_word(
    tx: &Transaction,
    w: &WordStatRecord,
    phase: u8,
    now_ms: u64,
) -> rusqlite::Result<()> {
    let existing = tx
        .query_row(
            "SELECT easiness_factor, interval_days, repetitions, avg_latency_ms, error_rate, last_seen
             FROM word_mastery WHERE word = ?1",
            params![w.word],
            |r| {
                Ok((
                    r.get::<_, f64>(0)? as f32,
                    r.get::<_, i64>(1)? as u32,
                    r.get::<_, i64>(2)? as u32,
                    r.get::<_, f64>(3)? as f32,
                    r.get::<_, f64>(4)? as f32,
                    r.get::<_, i64>(5)? as u64,
                ))
            },
        )
        .optional()?;

    let mut card = match existing {
        Some((ef, iv, rep, al, er, ls)) => Card {
            unit: w.word.clone(),
            easiness_factor: ef,
            interval: iv,
            repetitions: rep,
            avg_latency_ms: al,
            error_rate: er,
            last_seen: ls,
        },
        None => Card::new(&w.word),
    };

    card.review(
        w.error_rate,
        w.avg_latency_ms,
        target_latency_ms(phase),
        now_ms,
    );

    tx.execute(
        "INSERT INTO word_mastery
            (word, easiness_factor, interval_days, repetitions, avg_latency_ms, error_rate, next_review, last_seen)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         ON CONFLICT(word) DO UPDATE SET
            easiness_factor = excluded.easiness_factor,
            interval_days   = excluded.interval_days,
            repetitions     = excluded.repetitions,
            avg_latency_ms  = excluded.avg_latency_ms,
            error_rate      = excluded.error_rate,
            next_review     = excluded.next_review,
            last_seen       = excluded.last_seen",
        params![
            card.unit,
            f64::from(card.easiness_factor),
            card.interval,
            card.repetitions,
            f64::from(card.avg_latency_ms),
            f64::from(card.error_rate),
            card.next_review() as i64,
            card.last_seen as i64,
        ],
    )?;
    Ok(())
}

/// Inserts a new personal-best row if `value` exceeds the current best for
/// `metric`.
fn update_personal_best(
    tx: &Transaction,
    metric: &str,
    value: f64,
    session_id: i64,
    now_ms: u64,
) -> rusqlite::Result<()> {
    let current: Option<f64> = tx
        .query_row(
            "SELECT value FROM personal_bests WHERE metric = ?1 ORDER BY value DESC LIMIT 1",
            params![metric],
            |r| r.get(0),
        )
        .optional()?;

    if current.map(|c| value > c).unwrap_or(true) {
        tx.execute(
            "INSERT INTO personal_bests (metric, value, session_id, achieved_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![metric, value, session_id, now_ms as i64],
        )?;
    }
    Ok(())
}

/// Records the session against today's streak row (UTC date).
fn update_streak(tx: &Transaction, ended_at_ms: u64, duration_secs: u32) -> rusqlite::Result<()> {
    let date = utc_date(ended_at_ms);
    let minutes = (duration_secs / 60) as i64;
    tx.execute(
        "INSERT INTO streaks (date, sessions_count, total_minutes)
         VALUES (?1, 1, ?2)
         ON CONFLICT(date) DO UPDATE SET
            sessions_count = sessions_count + 1,
            total_minutes  = total_minutes + excluded.total_minutes",
        params![date, minutes],
    )?;
    Ok(())
}

/// Formats a Unix-epoch-millisecond instant as a UTC `YYYY-MM-DD` date.
fn utc_date(ms: u64) -> String {
    let days = (ms / 86_400_000) as i64;
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}-{m:02}-{d:02}")
}

/// Converts days since the Unix epoch to a civil `(year, month, day)`.
///
/// Howard Hinnant's well-known proleptic-Gregorian algorithm.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (y + i64::from(m <= 2), m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_date_known_values() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        // 2021-01-01 is 18628 days after the epoch.
        assert_eq!(civil_from_days(18_628), (2021, 1, 1));
    }

    #[test]
    fn utc_date_formats() {
        assert_eq!(utc_date(0), "1970-01-01");
    }

    fn sample_record(net_wpm: f64) -> SessionRecord {
        SessionRecord {
            started_at: 1_000,
            ended_at: 61_000,
            lesson_id: "free".into(),
            phase: 2,
            net_wpm,
            raw_wpm: net_wpm + 5.0,
            accuracy: 97.0,
            consistency: 85.0,
            total_chars: 300,
            error_count: 9,
            duration_secs: 60,
            corpus_type: "english_200.json".into(),
            key_stats: vec![KeyStatRecord {
                key_char: "a".into(),
                avg_latency_ms: 120.0,
                error_rate: 0.02,
                total_hits: 30,
            }],
            bigram_stats: vec![BigramStatRecord {
                bigram: "th".into(),
                avg_latency_ms: 110.0,
                error_rate: 0.0,
                total_hits: 12,
            }],
            word_stats: vec![WordStatRecord {
                word: "the".into(),
                error_rate: 0.0,
                avg_latency_ms: 100.0,
            }],
        }
    }

    fn count(conn: &Connection, table: &str) -> i64 {
        conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
            .unwrap()
    }

    #[test]
    fn end_to_end_persist_and_personal_best() {
        let path = std::env::temp_dir().join(format!(
            "typemaster_test_{}_{}.db",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let db = Db::open(&path).unwrap();
            db.save(sample_record(60.0));
            db.save(sample_record(80.0)); // higher net_wpm => new PB
            db.save(sample_record(50.0)); // lower => no new net_wpm PB
            db.close().await;
        });

        let conn = Connection::open(&path).unwrap();
        assert_eq!(count(&conn, "sessions"), 3);
        assert_eq!(count(&conn, "key_stats"), 3);
        assert_eq!(count(&conn, "bigram_stats"), 3);

        // word_mastery is upserted, so the single word collapses to one row whose
        // repetitions reflect all three reviews.
        let reps: i64 = conn
            .query_row(
                "SELECT repetitions FROM word_mastery WHERE word = 'the'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(reps, 3);

        // The best net_wpm seen is 80.
        let best_net: f64 = conn
            .query_row(
                "SELECT MAX(value) FROM personal_bests WHERE metric = 'net_wpm'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(best_net, 80.0);

        // Three identical-day sessions accumulate into one streak row.
        assert_eq!(count(&conn, "streaks"), 1);
        let sessions_today: i64 = conn
            .query_row("SELECT sessions_count FROM streaks", [], |r| r.get(0))
            .unwrap();
        assert_eq!(sessions_today, 3);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn lesson_progress_round_trips() {
        let path = std::env::temp_dir().join(format!(
            "typemaster_prog_{}_{}.db",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let db = Db::open(&path).unwrap();
            db.save_progress(ProgressRecord {
                lesson_id: "1.1".into(),
                consecutive_passes: 2,
                best_net_wpm: 22.0,
                completed: false,
            });
            // Upsert the same lesson: should overwrite, not duplicate.
            db.save_progress(ProgressRecord {
                lesson_id: "1.1".into(),
                consecutive_passes: 3,
                best_net_wpm: 25.0,
                completed: true,
            });
            db.save_progress(ProgressRecord {
                lesson_id: "1.2".into(),
                consecutive_passes: 1,
                best_net_wpm: 18.0,
                completed: false,
            });
            db.close().await;
        });

        // Reopen and load — simulates a fresh app launch. `Db::open` spawns a
        // task, so it must run inside the runtime context.
        let mut loaded = rt.block_on(async {
            let db2 = Db::open(&path).unwrap();
            db2.load_progress()
        });
        loaded.sort_by(|a, b| a.lesson_id.cmp(&b.lesson_id));
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].lesson_id, "1.1");
        assert_eq!(loaded[0].consecutive_passes, 3);
        assert!(loaded[0].completed);
        assert_eq!(loaded[0].best_net_wpm, 25.0);
        assert_eq!(loaded[1].lesson_id, "1.2");
        assert!(!loaded[1].completed);

        let _ = std::fs::remove_file(&path);
    }
}
