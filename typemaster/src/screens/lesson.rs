//! The active typing view: stats header, typing field, optional finger guide,
//! optional keyboard visualization, and a footer hint.

use engine::metrics::{finger_for, Finger};
use ratatui::layout::{Constraint, Direction, HorizontalAlignment, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::App;
use crate::components::keyboard_viz::keyboard_widget;
use crate::components::typing_field::typing_paragraph;
use crate::components::wpm_gauge::stats_header;

/// Renders the lesson screen for the active session.
pub fn render(frame: &mut Frame, area: Rect, app: &App, now_ms: u64, blink_on: bool) {
    let theme = app.theme();
    let session = app.session();

    // The current target character, used for finger guide + keyboard highlight.
    let current_char = session
        .target()
        .get(session.cursor())
        .and_then(|g| g.chars().next());

    // Assemble vertical sections, respecting the keyboard / finger-guide toggles.
    let mut constraints = vec![Constraint::Length(3), Constraint::Min(3)];
    if app.show_finger_guide() {
        constraints.push(Constraint::Length(1));
    }
    if app.show_keyboard() {
        constraints.push(Constraint::Length(6));
    }
    constraints.push(Constraint::Length(1));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    let mut idx = 0;
    frame.render_widget(stats_header(session, theme, now_ms), chunks[idx]);
    idx += 1;
    let ghost = app.ghost_index(now_ms);
    frame.render_widget(
        typing_paragraph(session, theme, blink_on, ghost, app.miss_flash()),
        chunks[idx],
    );
    idx += 1;

    if app.show_finger_guide() {
        frame.render_widget(finger_guide(theme, current_char), chunks[idx]);
        idx += 1;
    }
    if app.show_keyboard() {
        frame.render_widget(keyboard_widget(theme, current_char, None, 0.0), chunks[idx]);
        idx += 1;
    }

    let footer = Paragraph::new(Line::from(vec![Span::styled(
        " Esc: dashboard · Ctrl+R: restart · Ctrl+K: keyboard · Ctrl+F: fingers · Ctrl+G: ghost ",
        Style::default().fg(theme.text_muted),
    )]))
    .alignment(HorizontalAlignment::Center);
    frame.render_widget(footer, chunks[idx]);
}

/// One-line hint naming the finger for the upcoming key.
fn finger_guide(theme: &crate::themes::Theme, current: Option<char>) -> Paragraph<'static> {
    let label = Style::default().fg(theme.text_muted);
    let text = match current.and_then(finger_for) {
        Some(finger) => format!(
            "  Next key uses your {} ({})",
            finger_name(finger),
            finger.short_label()
        ),
        None => "  ".to_string(),
    };
    let style = match current.and_then(finger_for) {
        Some(finger) => Style::default().fg(theme.finger_color(finger)),
        None => label,
    };
    Paragraph::new(Line::from(Span::styled(text, style)))
}

/// Human-readable finger name for the guide.
fn finger_name(f: Finger) -> &'static str {
    match f {
        Finger::LeftPinky => "left pinky",
        Finger::LeftRing => "left ring",
        Finger::LeftMiddle => "left middle",
        Finger::LeftIndex => "left index",
        Finger::LeftThumb | Finger::RightThumb => "thumb",
        Finger::RightIndex => "right index",
        Finger::RightMiddle => "right middle",
        Finger::RightRing => "right ring",
        Finger::RightPinky => "right pinky",
    }
}
