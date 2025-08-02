//! Net-WPM history chart and local leaderboard.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::Frame;

use crate::app::App;
use crate::screens::footer;

/// Block characters for an inline sparkline, from empty to full.
const BARS: [char; 9] = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

/// Renders the progress screen.
pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let theme = app.theme();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // title
            Constraint::Length(5), // chart
            Constraint::Min(4),    // leaderboard
            Constraint::Length(1), // footer
        ])
        .split(area);

    let title = Paragraph::new(Line::from(Span::styled(
        "  Progress — net WPM over time",
        Style::default()
            .fg(theme.accent_cyan)
            .add_modifier(Modifier::BOLD),
    )));
    frame.render_widget(title, chunks[0]);

    let history = app.history();
    let chart_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .title(" history ");
    if history.is_empty() {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                " No sessions yet — type something!",
                Style::default().fg(theme.text_muted),
            )))
            .block(chart_block),
            chunks[1],
        );
    } else {
        let max = history.iter().cloned().fold(0.0_f64, f64::max).max(1.0);
        let spark: String = history
            .iter()
            .map(|&v| {
                let level = ((v / max) * (BARS.len() - 1) as f64).round() as usize;
                BARS[level.min(BARS.len() - 1)]
            })
            .collect();
        let last = history.last().copied().unwrap_or(0.0);
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(Span::styled(
                    format!(" {spark}"),
                    Style::default().fg(theme.accent_emerald),
                )),
                Line::from(vec![
                    Span::styled(" latest ", Style::default().fg(theme.text_muted)),
                    Span::styled(
                        format!("{last:.0} wpm"),
                        Style::default().fg(theme.text_primary),
                    ),
                    Span::styled("   peak ", Style::default().fg(theme.text_muted)),
                    Span::styled(
                        format!("{max:.0} wpm"),
                        Style::default().fg(theme.accent_gold),
                    ),
                ]),
            ])
            .block(chart_block),
            chunks[1],
        );
    }

    // Leaderboard.
    let mut lines: Vec<Line> = Vec::new();
    let board = app.leaderboard();
    if board.is_empty() {
        lines.push(Line::from(Span::styled(
            " No records yet.",
            Style::default().fg(theme.text_muted),
        )));
    } else {
        for (i, row) in board.iter().enumerate() {
            let rank_style = if i == 0 {
                Style::default()
                    .fg(theme.accent_gold)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.text_primary)
            };
            lines.push(Line::from(vec![
                Span::styled(
                    format!(" {:>2}. ", i + 1),
                    Style::default().fg(theme.text_muted),
                ),
                Span::styled(format!("{:>6.1} wpm", row.net_wpm), rank_style),
                Span::styled(
                    format!("   {:>5.1}% acc", row.accuracy),
                    Style::default().fg(theme.text_muted),
                ),
            ]));
        }
    }
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.border))
                .title(" leaderboard "),
        ),
        chunks[2],
    );

    frame.render_widget(
        footer(theme, &["Esc / q: back", "Ctrl+T: theme"]),
        chunks[3],
    );
}
