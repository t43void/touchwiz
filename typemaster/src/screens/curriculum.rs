//! The curriculum browser: phases, lessons, and lock/progress state.

use engine::adaptive::PASSES_TO_UNLOCK;
use engine::curriculum::Phase;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::Frame;

use crate::app::App;
use crate::screens::footer;

/// Renders the curriculum browser with a scrolling viewport around the cursor.
pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let theme = app.theme();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(3),
            Constraint::Length(1),
        ])
        .split(area);

    let title = Paragraph::new(vec![
        Line::from(Span::styled(
            "  Curriculum — 0 → 300 WPM",
            Style::default()
                .fg(theme.accent_cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "  complete a lesson 3× in a row to unlock the next",
            Style::default().fg(theme.text_muted),
        )),
    ]);
    frame.render_widget(title, chunks[0]);

    // Build all rows (phase headers + lessons), tracking which line is the
    // browse cursor so we can scroll it into view.
    let mut rows: Vec<Line> = Vec::new();
    let mut cursor_line = 0usize;
    let mut last_phase: Option<Phase> = None;

    for (i, lesson) in app.curriculum().lessons().iter().enumerate() {
        if last_phase != Some(lesson.phase) {
            if i != 0 {
                rows.push(Line::from(""));
            }
            rows.push(Line::from(Span::styled(
                format!("  {}", lesson.phase.title()),
                Style::default()
                    .fg(theme.accent_gold)
                    .add_modifier(Modifier::BOLD),
            )));
            last_phase = Some(lesson.phase);
        }

        let unlocked = app.curriculum().is_unlocked(i, app.progress());
        let prog = app.progress().get(&lesson.id);
        let completed = prog.map(|p| p.completed).unwrap_or(false);
        let passes = prog.map(|p| p.consecutive_passes).unwrap_or(0);
        let selected = i == app.browse_index();

        if selected {
            cursor_line = rows.len();
        }

        let (marker, marker_style) = if completed {
            ("✓", Style::default().fg(theme.accent_emerald))
        } else if !unlocked {
            ("locked", Style::default().fg(theme.text_muted))
        } else {
            ("○", Style::default().fg(theme.accent_cyan))
        };

        let name_style = if !unlocked {
            Style::default().fg(theme.text_muted)
        } else if selected {
            Style::default()
                .fg(theme.background)
                .bg(theme.accent_cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.text_primary)
        };

        let mut right = if completed {
            format!(
                "best {:.0} wpm",
                prog.map(|p| p.best_net_wpm).unwrap_or(0.0)
            )
        } else if unlocked {
            format!("{passes}/{PASSES_TO_UNLOCK}")
        } else {
            String::new()
        };
        if i == app.current_index() {
            right.push_str("  ← current");
        }

        rows.push(Line::from(vec![
            Span::styled(if selected { "  ▶ " } else { "    " }, marker_style),
            Span::styled(format!("{marker:<6} "), marker_style),
            Span::styled(format!("{:<5}", lesson.id), name_style),
            Span::styled(format!("{:<26}", lesson.title), name_style),
            Span::styled(right, Style::default().fg(theme.text_muted)),
        ]));
    }

    // Scroll so the cursor row stays visible.
    let list_area = chunks[1];
    let visible = list_area.height.saturating_sub(2) as usize;
    let offset = if rows.len() <= visible || visible == 0 {
        0
    } else {
        cursor_line
            .saturating_sub(visible / 2)
            .min(rows.len() - visible)
    };
    let end = (offset + visible).min(rows.len());
    let window: Vec<Line> = rows[offset..end].to_vec();

    let list = Paragraph::new(window).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border)),
    );
    frame.render_widget(list, list_area);

    frame.render_widget(
        footer(
            theme,
            &["↑/↓ move", "Enter start", "Ctrl+N/P next/prev", "Esc back"],
        ),
        chunks[2],
    );
}
