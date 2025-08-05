//! The core typing widget: renders the target text in three simultaneous
//! states (completed / current / upcoming) with per-character correctness
//! coloring and a block cursor (specification Section 6).

use engine::session::Session;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Wrap};

use crate::themes::Theme;

/// Builds the typing-field paragraph for the current session state.
///
/// - Completed characters are emerald (dimmed) if they were typed correctly,
///   or red if they were typed incorrectly.
/// - The current character carries a block cursor (cyan background).
/// - Upcoming characters are muted.
/// - When `ghost` is `Some(i)`, the character at `i` is marked (gold underline)
///   to show where a run at your best pace would be — race the ghost.
///
/// `blink_on` toggles the cursor visibility so the caller can animate it across
/// frames without this widget holding state.
pub fn typing_paragraph<'a>(
    session: &'a Session,
    theme: &Theme,
    blink_on: bool,
    ghost: Option<usize>,
) -> Paragraph<'a> {
    let cursor = session.cursor();
    let keystrokes = session.keystrokes();

    let completed = Style::default()
        .fg(theme.accent_emerald)
        .add_modifier(Modifier::DIM);
    let error = Style::default().fg(theme.error_red);
    let upcoming = Style::default().fg(theme.text_muted);
    let cursor_on = Style::default().fg(theme.background).bg(theme.accent_cyan);
    let cursor_off = Style::default()
        .fg(theme.text_primary)
        .add_modifier(Modifier::UNDERLINED);
    let ghost_style = Style::default()
        .fg(theme.accent_gold)
        .add_modifier(Modifier::UNDERLINED);

    let mut spans: Vec<Span> = Vec::with_capacity(session.total_chars());
    for (i, grapheme) in session.target().iter().enumerate() {
        let style = if i < cursor {
            match keystrokes.get(i) {
                Some(k) if k.correct => completed,
                _ => error,
            }
        } else if i == cursor {
            if blink_on {
                cursor_on
            } else {
                cursor_off
            }
        } else if ghost == Some(i) {
            ghost_style
        } else {
            upcoming
        };
        spans.push(Span::styled(grapheme.as_str(), style));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.background))
        .padding(ratatui::widgets::Padding::horizontal(1));

    Paragraph::new(Line::from(spans))
        .block(block)
        .wrap(Wrap { trim: false })
}
