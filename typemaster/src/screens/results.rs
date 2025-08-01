//! Post-session results: a centered, framed two-column breakdown.

use engine::adaptive::PASSES_TO_UNLOCK;
use engine::heatmap::Heatmap;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::App;
use crate::screens::{centered, footer, panel};
use crate::themes::Theme;

/// Renders the results screen for the most recent session.
pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let theme = app.theme();
    let session = app.session();
    let end = session.ended_at().unwrap_or(0);

    let muted = Style::default().fg(theme.text_muted);
    let value = Style::default()
        .fg(theme.text_primary)
        .add_modifier(Modifier::BOLD);
    let gold = Style::default()
        .fg(theme.accent_gold)
        .add_modifier(Modifier::BOLD);

    let block = panel(theme, "Results");
    let outer = centered(area, 76, 18);
    let inner = block.inner(outer);
    frame.render_widget(block, outer);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // banner
            Constraint::Length(6), // two-column body
            Constraint::Length(1), // gap
            Constraint::Length(2), // bigrams
            Constraint::Min(0),    // flex
            Constraint::Length(1), // footer
        ])
        .split(inner);

    frame.render_widget(banner(app, theme), rows[0]);

    // Two columns: metrics on the left, slowest keys on the right.
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[1]);

    let metrics = Paragraph::new(vec![
        metric(
            "Net WPM",
            format!("{:.1}", session.net_wpm(end)),
            muted,
            gold,
        ),
        metric(
            "Raw WPM",
            format!("{:.1}", session.raw_wpm(end)),
            muted,
            value,
        ),
        metric(
            "Accuracy",
            format!("{:.1}%", session.accuracy()),
            muted,
            value,
        ),
        metric(
            "Consistency",
            format!("{:.0}/100", session.consistency()),
            muted,
            value,
        ),
        metric("Time", fmt_secs(session.elapsed_ms(end)), muted, value),
        metric("Errors", session.error_count().to_string(), muted, value),
    ]);
    frame.render_widget(metrics, cols[0]);

    let hm = Heatmap::from_session(session);
    let mut key_lines = vec![Line::from(Span::styled("Slowest keys", muted))];
    for (k, ms) in hm.slowest_keys(5) {
        let shown = if k == ' ' {
            "spc".to_string()
        } else {
            k.to_string()
        };
        key_lines.push(Line::from(vec![
            Span::styled(format!("  {shown:<4}"), value),
            Span::styled(format!("{ms:>5.0} ms"), muted),
        ]));
    }
    frame.render_widget(Paragraph::new(key_lines), cols[1]);

    // Most-errored bigrams on one centered line.
    let errored = hm.most_errored_bigrams(5);
    let mut bigram_spans = vec![Span::styled("Most-errored: ", muted)];
    if errored.is_empty() {
        bigram_spans.push(Span::styled("none — clean run!", value));
    } else {
        for (i, (b, rate)) in errored.iter().enumerate() {
            if i > 0 {
                bigram_spans.push(Span::styled("   ", muted));
            }
            bigram_spans.push(Span::styled(format!("{b} "), value));
            bigram_spans.push(Span::styled(format!("{:.0}%", rate * 100.0), muted));
        }
    }
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(""),
            Line::from(bigram_spans).alignment(Alignment::Center),
        ]),
        rows[3],
    );

    frame.render_widget(
        footer(
            theme,
            &[
                "Enter dashboard",
                "Ctrl+R again",
                "Ctrl+N next",
                "Ctrl+H heatmap",
            ],
        ),
        rows[5],
    );
}

/// The pass/fail banner (two centered lines).
fn banner<'a>(app: &'a App, theme: &Theme) -> Paragraph<'a> {
    let lesson = app.current_lesson();
    let (title, subtitle, color) = match app.last_outcome() {
        Some(o) if o.newly_completed => (
            format!("✓  {} — LESSON COMPLETE", lesson.title),
            "next lesson unlocked".to_string(),
            theme.accent_gold,
        ),
        Some(o) if o.passed => (
            format!("✓  {} — PASS", lesson.title),
            format!(
                "{} / {} consecutive",
                o.consecutive_passes, PASSES_TO_UNLOCK
            ),
            theme.accent_emerald,
        ),
        Some(_) => (
            format!("✗  {} — keep going", lesson.title),
            format!(
                "need {:.0} wpm @ {:.0}% accuracy",
                lesson.pass.required_wpm, lesson.pass.required_accuracy
            ),
            theme.error_red,
        ),
        None => (
            format!("{} — complete", lesson.title),
            "custom session".to_string(),
            theme.accent_cyan,
        ),
    };

    Paragraph::new(vec![
        Line::from(Span::styled(
            title,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ))
        .alignment(Alignment::Center),
        Line::from(Span::styled(
            subtitle,
            Style::default().fg(theme.text_muted),
        ))
        .alignment(Alignment::Center),
    ])
}

/// A single label/value metric line.
fn metric<'a>(label: &'a str, val: String, label_style: Style, val_style: Style) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("{label:<13}"), label_style),
        Span::styled(val, val_style),
    ])
}

fn fmt_secs(ms: u64) -> String {
    let secs = ms / 1000;
    format!("{}:{:02}", secs / 60, secs % 60)
}
