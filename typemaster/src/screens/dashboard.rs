//! Home screen: centered title, headline stats, today's streak, and the menu.

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::{App, MENU_ITEMS};
use crate::screens::{centered, footer, panel};

/// Renders the dashboard.
pub fn render(frame: &mut Frame, area: Rect, app: &App, _now_ms: u64) {
    let theme = app.theme();

    // A centered content column keeps the home screen balanced at any width.
    let content = centered(area, 60, 22);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // title
            Constraint::Length(7), // stats
            Constraint::Length(8), // menu
            Constraint::Length(1), // footer
        ])
        .split(content);

    // Title.
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(
                "T Y P E M A S T E R",
                Style::default()
                    .fg(theme.accent_cyan)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "master the keyboard · 0 → 300 WPM",
                Style::default().fg(theme.text_muted),
            )),
        ])
        .alignment(Alignment::Center),
        chunks[0],
    );

    // Stats.
    let s = app.stats();
    let label = Style::default().fg(theme.text_muted);
    let value = Style::default()
        .fg(theme.text_primary)
        .add_modifier(Modifier::BOLD);
    let gold = Style::default()
        .fg(theme.accent_gold)
        .add_modifier(Modifier::BOLD);

    frame.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled(format!("{:<14}", "Best WPM"), label),
                Span::styled(format!("{:>6.1}", s.best_net_wpm), gold),
                Span::styled(format!("    {:<14}", "Best accuracy"), label),
                Span::styled(format!("{:>5.1}%", s.best_accuracy), value),
            ]),
            Line::from(vec![
                Span::styled(format!("{:<14}", "Last WPM"), label),
                Span::styled(format!("{:>6.1}", s.last_net_wpm), value),
                Span::styled(format!("    {:<14}", "Total sessions"), label),
                Span::styled(format!("{:>6}", s.total_sessions), value),
            ]),
            Line::from(vec![
                Span::styled(format!("{:<14}", "Today"), label),
                Span::styled(
                    format!("{} sessions · {} min", s.today_sessions, s.today_minutes),
                    value,
                ),
            ]),
        ])
        .block(panel(theme, "stats")),
        chunks[1],
    );

    // Menu.
    let mut menu_lines: Vec<Line> = Vec::new();
    for (i, item) in MENU_ITEMS.iter().enumerate() {
        let selected = i == app.menu_idx();
        let style = if selected {
            Style::default()
                .fg(theme.accent_cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.text_primary)
        };
        let marker = if selected { "▶  " } else { "   " };
        menu_lines.push(Line::from(vec![
            Span::styled(marker, style),
            Span::styled(*item, style),
        ]));
    }
    frame.render_widget(
        Paragraph::new(menu_lines).block(panel(theme, "menu")),
        chunks[2],
    );

    frame.render_widget(
        footer(
            theme,
            &[
                "↑/↓ move",
                "Enter select",
                "Ctrl+T theme",
                "? help",
                "q quit",
            ],
        ),
        chunks[3],
    );
}
