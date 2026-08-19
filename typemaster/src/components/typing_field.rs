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
/// - Completed characters are emerald (dimmed); the engine stays on error so
///   completed positions are always correct advances.
/// - The current character carries a block cursor (cyan background), or a red
///   flash when `miss_flash` is set after a wrong key.
/// - Upcoming characters are muted.
/// - When `ghost` is `Some(i)`, that character is marked (gold underline)
///   to show where a run at your best pace would be — race the ghost.
///
/// `blink_on` toggles the cursor visibility so the caller can animate it across
/// frames without this widget holding state.
pub fn typing_paragraph<'a>(
    session: &'a Session,
    theme: &Theme,
    blink_on: bool,
    ghost: Option<usize>,
    miss_flash: bool,
) -> Paragraph<'a> {
    let cursor = session.cursor();

    let completed = Style::default()
        .fg(theme.accent_emerald)
        .add_modifier(Modifier::DIM);
    let upcoming = Style::default().fg(theme.text_muted);
    let cursor_on = Style::default().fg(theme.background).bg(theme.accent_cyan);
    let cursor_off = Style::default()
        .fg(theme.text_primary)
        .add_modifier(Modifier::UNDERLINED);
    let miss_style = Style::default()
        .fg(theme.background)
        .bg(theme.error_red)
        .add_modifier(Modifier::BOLD);
    let ghost_style = Style::default()
        .fg(theme.accent_gold)
        .add_modifier(Modifier::UNDERLINED);

    let mut spans: Vec<Span> = Vec::with_capacity(session.total_chars());
    for (i, grapheme) in session.target().iter().enumerate() {
        let at_ghost = ghost == Some(i);
        let style = if i < cursor {
            if at_ghost {
                ghost_style
            } else {
                completed
            }
        } else if i == cursor {
            if miss_flash {
                miss_style
            } else if blink_on {
                cursor_on
            } else if at_ghost {
                ghost_style
            } else {
                cursor_off
            }
        } else if at_ghost {
            ghost_style
        } else {
            upcoming
        };
        spans.push(Span::styled(grapheme.as_str(), style));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(if miss_flash {
            theme.error_red
        } else {
            theme.border
        }))
        .style(Style::default().bg(theme.background))
        .padding(ratatui::widgets::Padding::horizontal(1));

    Paragraph::new(Line::from(spans))
        .block(block)
        .wrap(Wrap { trim: false })
}
