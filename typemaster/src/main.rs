//! touchwiz binary entry point: terminal lifecycle and the render/event loop.

mod app;
mod components;
mod config;
mod db;
mod screens;
mod themes;

use std::io::{self, Stdout, Write};
use std::path::PathBuf;
use std::time::Duration;

use clap::{CommandFactory, Parser};
use crossterm::event::{self, Event, KeyEventKind};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::{cursor, execute};
use engine::import::prepare_text;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::app::{now_ms, App, KeyOutcome};
use crate::db::{Db, SessionRecord};

/// touchwiz — a terminal-first typing trainer.
#[derive(Debug, Parser)]
#[command(name = "touchwiz", version, about)]
struct Cli {
    /// Train on a custom text or code file instead of the curriculum.
    #[arg(long, value_name = "PATH")]
    file: Option<PathBuf>,

    /// When importing with --file, strip // # and /* */ comments.
    #[arg(long, requires = "file")]
    strip_comments: bool,

    /// Generate the man page into the given directory and exit.
    #[arg(long, value_name = "DIR", hide = true)]
    generate_man: Option<PathBuf>,

    /// Render every screen to stdout as text and exit (no terminal needed).
    #[arg(long, hide = true)]
    screenshot: bool,
}

/// Event poll timeout: ~60fps (specification Quality rule 9).
const FRAME_TIMEOUT: Duration = Duration::from_millis(16);

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    if let Some(dir) = cli.generate_man.as_deref() {
        generate_man_page(dir)?;
        return Ok(());
    }

    if cli.screenshot {
        print_screenshots();
        return Ok(());
    }

    let mut app = App::new()?;

    // Persistence is best-effort: a missing data dir or DB error must not stop
    // the user from typing.
    let db = match config::database_path() {
        Some(path) => match Db::open(&path) {
            Ok(db) => Some(db),
            Err(e) => {
                eprintln!("touchwiz: persistence disabled ({e})");
                None
            }
        },
        None => None,
    };

    app.init(db.as_ref(), now_ms());

    // A --file launch jumps straight into a custom session; otherwise show the
    // splash.
    match cli.file.as_deref() {
        Some(path) => {
            let raw = std::fs::read_to_string(path)?;
            let text = prepare_text(&raw, cli.strip_comments);
            if text.trim().is_empty() {
                return Err(format!("no usable text in {}", path.display()).into());
            }
            app.load_custom(text);
        }
        None => app.show_splash(),
    }

    install_panic_hook();
    let mut terminal = setup_terminal()?;
    let result = run(&mut terminal, &mut app, db.as_ref());
    restore_terminal()?;

    if let Some(db) = db {
        db.close().await;
    }
    result
}

/// The render/event loop. Returns once the user quits.
fn run(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
    db: Option<&Db>,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        app.tick();
        let now = now_ms();
        let blink = app.cursor_blink_on();
        terminal.draw(|frame| {
            screens::render(frame, app, now, blink);
        })?;

        if event::poll(FRAME_TIMEOUT)? {
            if let Event::Key(key) = event::read()? {
                // Ignore key-release/repeat events that some platforms emit.
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match app.on_key(key, now_ms(), db) {
                    KeyOutcome::Quit => break,
                    KeyOutcome::SessionFinished => {
                        if let Some(db) = db {
                            db.save(SessionRecord::from_session(
                                app.session(),
                                app.current_lesson(),
                            ));
                            if !app.is_custom() {
                                db.save_progress(app.current_progress_record());
                            }
                        }
                    }
                    KeyOutcome::None => {}
                }
                // Emit a terminal bell for an error keystroke when audio is on.
                if app.take_bell() {
                    let mut out = io::stdout();
                    let _ = out.write_all(b"\x07");
                    let _ = out.flush();
                }
            }
        }
    }
    Ok(())
}

/// Writes the clap-generated man page to `dir/touchwiz.1`.
fn generate_man_page(dir: &std::path::Path) -> io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let man = clap_mangen::Man::new(Cli::command());
    let mut buf = Vec::new();
    man.render(&mut buf)?;
    let path = dir.join("touchwiz.1");
    std::fs::write(&path, buf)?;
    println!("wrote {}", path.display());
    Ok(())
}

/// Renders `app` to an off-screen buffer and returns it as plain text.
fn render_to_text(app: &App, now: u64, w: u16, h: u16) -> String {
    use ratatui::backend::TestBackend;
    use ratatui::layout::Position;

    let mut term = match Terminal::new(TestBackend::new(w, h)) {
        Ok(t) => t,
        Err(_) => return String::new(),
    };
    if term
        .draw(|frame| screens::render(frame, app, now, true))
        .is_err()
    {
        return String::new();
    }
    let buf = term.backend().buffer();
    let area = buf.area;
    let mut out = String::new();
    for y in 0..area.height {
        let mut line = String::new();
        for x in 0..area.width {
            if let Some(cell) = buf.cell(Position::new(x, y)) {
                line.push_str(cell.symbol());
            }
        }
        out.push_str(line.trim_end());
        out.push('\n');
    }
    out
}

/// Renders each screen to stdout as text — a no-terminal preview of the app.
fn print_screenshots() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    let key = |code: KeyCode| KeyEvent::new(code, KeyModifiers::NONE);
    let ch = |c: char| KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE);
    let ctrl = |c: char| KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL);
    let (w, h) = (94u16, 28u16);
    let banner =
        |name: &str| println!("\n========================  {name}  ========================\n");

    let type_text = |app: &mut App, text: &str| -> u64 {
        let mut now = 1_000u64;
        for c in text.chars() {
            now += 90;
            app.on_key(
                KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE),
                now,
                None,
            );
        }
        now
    };

    // Splash (partially revealed).
    if let Ok(mut a) = App::new() {
        a.show_splash();
        for _ in 0..32 {
            a.tick();
        }
        banner("Splash");
        print!("{}", render_to_text(&a, 0, w, h));
    }

    // Dashboard.
    if let Ok(a) = App::new() {
        banner("Dashboard");
        print!("{}", render_to_text(&a, 0, w, h));
    }

    // Curriculum browser.
    if let Ok(mut a) = App::new() {
        a.on_key(key(KeyCode::Enter), 0, None);
        a.on_key(key(KeyCode::Down), 0, None);
        banner("Curriculum");
        print!("{}", render_to_text(&a, 0, w, h));
    }

    // Lesson (finger guide + keyboard, mid-type).
    if let Ok(mut a) = App::new() {
        a.on_key(ch('f'), 0, None); // finger guide on
        a.load_custom("the quick brown fox jumps over the lazy dog".to_string());
        let now = type_text(&mut a, "the quick br");
        banner("Lesson");
        print!("{}", render_to_text(&a, now, w, h));
    }

    // Results.
    if let Ok(mut a) = App::new() {
        a.load_custom("hello world foo bar baz".to_string());
        let target: String = a.session().target().concat();
        let now = type_text(&mut a, &target);
        banner("Results");
        print!("{}", render_to_text(&a, now, w, h));

        // Heatmap of that session.
        a.on_key(ctrl('h'), 0, None);
        banner("Heatmap");
        print!("{}", render_to_text(&a, now, w, h));
    }

    // Settings.
    if let Ok(mut a) = App::new() {
        a.on_key(ctrl('s'), 0, None);
        banner("Settings");
        print!("{}", render_to_text(&a, 0, w, h));
    }

    // Help overlay (over the dashboard).
    if let Ok(mut a) = App::new() {
        a.on_key(ch('?'), 0, None);
        banner("Help overlay");
        print!("{}", render_to_text(&a, 0, w, h));
    }
}

fn setup_terminal() -> io::Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, cursor::Hide)?;
    Terminal::new(CrosstermBackend::new(stdout))
}

/// Restores the terminal to its normal state. Safe to call multiple times.
fn restore_terminal() -> io::Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen, cursor::Show)?;
    Ok(())
}

/// Installs a panic hook that restores the terminal before propagating, so a
/// crash never leaves the user's terminal in raw mode (Quality rule 11).
fn install_panic_hook() {
    let original = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = restore_terminal();
        original(info);
    }));
}

#[cfg(test)]
mod render_tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    use crate::app::App;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }
    fn ctrl(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
    }

    /// Renders the app at both the minimal and full layout widths, asserting the
    /// draw never panics.
    fn render_both(app: &App) {
        for (w, h) in [(80u16, 24u16), (120, 40)] {
            let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
            terminal
                .draw(|frame| crate::screens::render(frame, app, 5_000, true))
                .unwrap();
        }
    }

    #[test]
    fn every_screen_renders_without_panicking() {
        // Splash.
        let mut app = App::new().unwrap();
        app.show_splash();
        render_both(&app);

        // Dashboard.
        let mut app = App::new().unwrap();
        render_both(&app);

        // Help overlay over the dashboard.
        app.on_key(key(KeyCode::Char('?')), 0, None);
        render_both(&app);
        app.on_key(key(KeyCode::Esc), 0, None);

        // Curriculum browser.
        app.on_key(key(KeyCode::Enter), 0, None);
        render_both(&app);

        // Settings, heatmap (no session yet), progress.
        app.on_key(ctrl('s'), 0, None);
        render_both(&app);
        app.on_key(key(KeyCode::Esc), 0, None);
        app.on_key(ctrl('h'), 0, None);
        render_both(&app);

        // Custom lesson + ghost + finger guide on, then play to results.
        let mut app = App::new().unwrap();
        app.on_key(key(KeyCode::Char('g')), 0, None); // ghost on
        app.on_key(key(KeyCode::Char('f')), 0, None); // finger guide on
        app.load_custom("the quick brown fox".to_string());
        render_both(&app); // Lesson screen
        let target: String = app.session().target().concat();
        let mut now = 1_000;
        for g in target.chars() {
            now += 40;
            app.on_key(
                KeyEvent::new(KeyCode::Char(g), KeyModifiers::NONE),
                now,
                None,
            );
        }
        render_both(&app); // Results screen
        app.on_key(ctrl('h'), 0, None);
        render_both(&app); // Heatmap with a real session
    }
}
