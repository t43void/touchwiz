//! ASCII QWERTY keyboard visualization (specification Section 6).
//!
//! In lesson mode each key is colored by its finger zone, with the current key
//! highlighted. Given a [`Heatmap`], keys are instead colored by health
//! (green/yellow/red) for the post-session error heatmap.

use engine::heatmap::{Heatmap, KeyHealth};
use engine::metrics::finger_for;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

use crate::themes::Theme;

/// Physical key rows (lowercase), staggered to approximate a real keyboard.
const ROWS: [&str; 4] = ["1234567890", "qwertyuiop", "asdfghjkl;", "zxcvbnm,./"];
/// Leading indentation per row, mimicking the QWERTY stagger.
const INDENT: [usize; 4] = [0, 1, 2, 3];

/// Builds the keyboard widget.
///
/// - `highlight`: the key to emphasize (the current target character).
/// - `heatmap` + `latency_threshold_ms`: when present, color by key health
///   instead of finger zone.
pub fn keyboard_widget<'a>(
    theme: &Theme,
    highlight: Option<char>,
    heatmap: Option<&Heatmap>,
    latency_threshold_ms: f64,
) -> Paragraph<'a> {
    let mut lines: Vec<Line> = Vec::with_capacity(ROWS.len());
    for (row, indent) in ROWS.iter().zip(INDENT.iter()) {
        let mut spans: Vec<Span> = vec![Span::raw(" ".repeat(*indent))];
        for c in row.chars() {
            let style = key_style(theme, c, highlight, heatmap, latency_threshold_ms);
            spans.push(Span::styled(format!(" {} ", c.to_uppercase()), style));
        }
        lines.push(Line::from(spans));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.background));
    Paragraph::new(lines).block(block)
}

fn key_style(
    theme: &Theme,
    c: char,
    highlight: Option<char>,
    heatmap: Option<&Heatmap>,
    latency_threshold_ms: f64,
) -> Style {
    if highlight == Some(c) {
        return Style::default()
            .fg(theme.background)
            .bg(theme.accent_cyan)
            .add_modifier(Modifier::BOLD);
    }
    if let Some(hm) = heatmap {
        let color = match hm.key(c).map(|s| s.health(latency_threshold_ms)) {
            Some(KeyHealth::Good) => theme.accent_emerald,
            Some(KeyHealth::Warning) => theme.warning_amber,
            Some(KeyHealth::Bad) => theme.error_red,
            None => theme.text_muted,
        };
        return Style::default().fg(color);
    }
    match finger_for(c) {
        Some(f) => Style::default().fg(theme.finger_color(f)),
        None => Style::default().fg(theme.text_muted),
    }
}
