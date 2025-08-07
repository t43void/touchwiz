//! Live statistics header: net WPM, accuracy, elapsed time, and consistency
//! (specification Section 6, 80-column layout).

use engine::session::Session;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

use crate::themes::Theme;

/// Formats milliseconds as `M:SS`.
fn fmt_clock(elapsed_ms: u64) -> String {
    let secs = elapsed_ms / 1000;
    format!("{}:{:02}", secs / 60, secs % 60)
}

/// Builds the live stats header paragraph for the session as of `now_ms`.
pub fn stats_header<'a>(session: &Session, theme: &Theme, now_ms: u64) -> Paragraph<'a> {
    let label = Style::default().fg(theme.text_muted);
    let value = Style::default()
        .fg(theme.text_primary)
        .add_modifier(Modifier::BOLD);
    let wpm_value = Style::default()
        .fg(theme.accent_cyan)
        .add_modifier(Modifier::BOLD);

    // Smoothed live WPM over a trailing 10-second window (Section 5).
    let net = session.windowed_net_wpm(now_ms, 10_000);
    let acc = session.accuracy();
    let consistency = session.consistency();
    let clock = fmt_clock(session.elapsed_ms(now_ms));

    // Warn when accuracy slips below the 95% pass floor.
    let acc_style = if acc < 95.0 {
        Style::default()
            .fg(theme.warning_amber)
            .add_modifier(Modifier::BOLD)
    } else {
        value
    };

    let spans = vec![
        Span::styled(" WPM: ", label),
        Span::styled(format!("{net:>3.0}"), wpm_value),
        Span::styled("   ACC: ", label),
        Span::styled(format!("{acc:>4.1}%"), acc_style),
        Span::styled("   TIME: ", label),
        Span::styled(clock, value),
        Span::styled("   CONSISTENCY: ", label),
        Span::styled(format!("{consistency:>3.0}"), value),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.surface))
        .padding(ratatui::widgets::Padding::horizontal(1));

    Paragraph::new(Line::from(spans)).block(block)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_formatting() {
        assert_eq!(fmt_clock(0), "0:00");
        assert_eq!(fmt_clock(9_000), "0:09");
        assert_eq!(fmt_clock(83_000), "1:23");
    }
}
