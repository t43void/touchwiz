//! Per-key error/latency heatmap of the last session.

use engine::adaptive::target_latency_ms;
use engine::heatmap::Heatmap;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::App;
use crate::components::keyboard_viz::keyboard_widget;

/// Renders the heatmap screen.
pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let theme = app.theme();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // title
            Constraint::Length(6), // keyboard
            Constraint::Length(3), // legend
            Constraint::Min(1),    // footer
        ])
        .split(area);

    let title = Paragraph::new(Line::from(Span::styled(
        "  Keyboard heatmap — last session",
        Style::default()
            .fg(theme.accent_cyan)
            .add_modifier(Modifier::BOLD),
    )));
    frame.render_widget(title, chunks[0]);

    if app.has_session() {
        let hm = Heatmap::from_session(app.session());
        let threshold = f64::from(target_latency_ms(app.current_lesson().phase.number()));
        frame.render_widget(
            keyboard_widget(theme, None, Some(&hm), threshold),
            chunks[1],
        );

        let legend = Paragraph::new(Line::from(vec![
            Span::styled("  ███ ", Style::default().fg(theme.accent_emerald)),
            Span::styled("good   ", Style::default().fg(theme.text_muted)),
            Span::styled("███ ", Style::default().fg(theme.warning_amber)),
            Span::styled(
                "slow / few errors   ",
                Style::default().fg(theme.text_muted),
            ),
            Span::styled("███ ", Style::default().fg(theme.error_red)),
            Span::styled("needs work   ", Style::default().fg(theme.text_muted)),
            Span::styled("███ ", Style::default().fg(theme.text_muted)),
            Span::styled("untyped", Style::default().fg(theme.text_muted)),
        ]))
        .block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(theme.border)),
        );
        frame.render_widget(legend, chunks[2]);
    } else {
        let msg = Paragraph::new(Line::from(Span::styled(
            "  Complete a session to see your heatmap.",
            Style::default().fg(theme.text_muted),
        )));
        frame.render_widget(msg, chunks[1]);
    }

    frame.render_widget(
        crate::screens::footer(theme, &["Esc / q: back", "Ctrl+T: theme"]),
        chunks[3],
    );
}
