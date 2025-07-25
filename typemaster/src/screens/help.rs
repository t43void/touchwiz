//! Keybindings help overlay (toggled with `?`).

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};
use ratatui::Frame;

use crate::app::App;
use crate::screens::{centered, panel};

/// Keybinding rows: (keys, description).
const BINDINGS: [(&str, &str); 14] = [
    ("type", "begin / enter characters"),
    ("Esc / q", "back (quit from dashboard)"),
    ("Enter", "confirm / select / start"),
    ("Tab ↑ ↓", "move selection"),
    ("Ctrl+N / Ctrl+P", "next / previous lesson"),
    ("Ctrl+R", "restart current session"),
    ("Ctrl+H", "show heatmap"),
    ("Ctrl+S", "open settings"),
    ("Ctrl+T", "cycle theme"),
    ("k / Ctrl+K", "toggle keyboard view"),
    ("f / Ctrl+F", "toggle finger guide"),
    ("g / Ctrl+G", "toggle ghost racer"),
    ("m", "toggle audio feedback"),
    ("?", "toggle this help"),
];

/// Renders the help overlay centered over the current screen.
pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let theme = app.theme();
    let key_style = Style::default()
        .fg(theme.accent_cyan)
        .add_modifier(Modifier::BOLD);
    let desc_style = Style::default().fg(theme.text_primary);

    let mut lines: Vec<Line> = Vec::new();
    for (keys, desc) in BINDINGS {
        lines.push(Line::from(vec![
            Span::styled(format!("{keys:<17}"), key_style),
            Span::styled(desc, desc_style),
        ]));
    }
    lines.push(Line::from(""));
    lines.push(
        Line::from(Span::styled(
            "Esc / ? to close",
            Style::default().fg(theme.text_muted),
        ))
        .centered(),
    );

    let block = panel(theme, "Keybindings");
    let rect = centered(area, 58, lines.len() as u16 + 4);
    frame.render_widget(Clear, rect);
    frame.render_widget(Paragraph::new(lines).block(block), rect);
}
