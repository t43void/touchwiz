//! Full-screen views and the top-level render dispatcher.

pub mod curriculum;
pub mod dashboard;
pub mod heatmap;
pub mod help;
pub mod lesson;
pub mod progress;
pub mod results;
pub mod settings;
pub mod splash;

use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Padding, Paragraph};
use ratatui::Frame;

use crate::app::{App, Screen};
use crate::themes::Theme;

/// Renders the active screen (plus the help overlay when shown).
pub fn render(frame: &mut Frame, app: &App, now_ms: u64, blink_on: bool) {
    let area = frame.area();
    // Theme background fill behind every screen.
    frame.render_widget(
        Block::default().style(Style::default().bg(app.theme().background)),
        area,
    );

    match app.screen() {
        Screen::Splash => splash::render(frame, area, app),
        Screen::Dashboard => dashboard::render(frame, area, app, now_ms),
        Screen::Curriculum => curriculum::render(frame, area, app),
        Screen::Lesson => lesson::render(frame, area, app, now_ms, blink_on),
        Screen::Results => results::render(frame, area, app),
        Screen::Heatmap => heatmap::render(frame, area, app),
        Screen::Progress => progress::render(frame, area, app),
        Screen::Settings => settings::render(frame, area, app),
    }

    if app.show_help() {
        help::render(frame, area, app);
    }
}

/// Computes a centered rectangle of the given width/height within `area`.
pub(crate) fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let w = width.min(area.width);
    let h = height.min(area.height);
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    Rect::new(x, y, w, h)
}

/// A rounded, padded panel with a centered title — the house style for all
/// framed surfaces.
pub(crate) fn panel<'a>(theme: &Theme, title: &'a str) -> Block<'a> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .title(
            Line::from(Span::styled(
                format!(" {title} "),
                Style::default()
                    .fg(theme.accent_cyan)
                    .add_modifier(Modifier::BOLD),
            ))
            .centered(),
        )
        .style(Style::default().bg(theme.surface))
        .padding(Padding::symmetric(2, 1))
}

/// A centered footer with hints joined by dot separators (no dangling dots).
pub(crate) fn footer<'a>(theme: &Theme, items: &[&'a str]) -> Paragraph<'a> {
    let mut spans: Vec<Span> = Vec::new();
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  ·  ", Style::default().fg(theme.border)));
        }
        spans.push(Span::styled(*item, Style::default().fg(theme.text_muted)));
    }
    Paragraph::new(Line::from(spans)).alignment(Alignment::Center)
}
