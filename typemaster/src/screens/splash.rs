//! Animated launch splash: the ASCII logo fades/reveals in, then auto-advances.

use ratatui::layout::HorizontalAlignment;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::App;
use crate::screens::centered;

/// ASCII logo, revealed line by line as the splash animates.
const LOGO: [&str; 6] = [
    "  ████████ ██    ██ ██████  ███████",
    "     ██     ██  ██  ██   ██ ██     ",
    "     ██      ████   ██████  █████  ",
    "     ██       ██    ██      ██     ",
    "     ██       ██    ██      ███████",
    "        T Y P E M A S T E R        ",
];

/// Renders the splash screen, revealing logo lines as frames advance.
pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let theme = app.theme();
    // Reveal one logo line roughly every 8 frames.
    let revealed = (app.frame() / 8) as usize + 1;

    let mut lines: Vec<Line> = Vec::new();
    for (i, row) in LOGO.iter().enumerate() {
        let style = if i + 1 == LOGO.len() {
            Style::default()
                .fg(theme.accent_gold)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(theme.accent_cyan)
                .add_modifier(Modifier::BOLD)
        };
        let text = if i < revealed { *row } else { "" };
        lines.push(Line::from(Span::styled(text, style)));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "press any key",
        Style::default().fg(theme.text_muted),
    )));

    let panel = centered(area, 40, lines.len() as u16);
    frame.render_widget(
        Paragraph::new(lines).alignment(HorizontalAlignment::Center),
        panel,
    );
}
