//! Settings: theme and display toggles, shown in a centered framed panel.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::App;
use crate::screens::{centered, footer, panel};

/// Renders the settings screen.
pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let theme = app.theme();
    let label = Style::default().fg(theme.text_muted);
    let value = Style::default()
        .fg(theme.text_primary)
        .add_modifier(Modifier::BOLD);
    let on = Style::default()
        .fg(theme.accent_emerald)
        .add_modifier(Modifier::BOLD);
    let off = Style::default().fg(theme.text_muted);
    let warn = Style::default()
        .fg(theme.error_red)
        .add_modifier(Modifier::BOLD);
    let toggle = |b: bool| if b { ("on", on) } else { ("off", off) };

    let (kbd, kbd_s) = toggle(app.show_keyboard());
    let (fg, fg_s) = toggle(app.show_finger_guide());
    let (ghost, ghost_s) = toggle(app.show_ghost());
    let (audio, audio_s) = toggle(app.audio_enabled());

    let row = |name: &'static str, val: &'static str, vs: Style, hint: &'static str| {
        Line::from(vec![
            Span::styled(format!("  {name:<16}"), label),
            Span::styled(format!("{val:<5}"), vs),
            Span::styled(hint, label),
        ])
    };

    let reset_line = if app.reset_armed() {
        Line::from(vec![
            Span::styled(format!("  {:<16}", "Reset progress"), label),
            Span::styled("CONFIRM", warn),
            Span::styled("  Ctrl+X again · Esc cancel", label),
        ])
    } else {
        row("Reset progress", "—", value, "Ctrl+X (twice)")
    };

    let block = panel(theme, "Settings");
    let outer = centered(area, 64, 16);
    let inner = block.inner(outer);
    frame.render_widget(block, outer);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(7),
            Constraint::Length(2),
            Constraint::Length(1),
        ])
        .split(inner);

    frame.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled(format!("  {:<16}", "Theme"), label),
                Span::styled(format!("{:<5}", theme.name), value),
                Span::styled("Ctrl+T to cycle", label),
            ]),
            row("Keyboard view", kbd, kbd_s, "k to toggle"),
            row("Finger guide", fg, fg_s, "f to toggle"),
            row("Ghost racer", ghost, ghost_s, "g to toggle"),
            row("Audio feedback", audio, audio_s, "m / Ctrl+M"),
            reset_line,
        ]),
        rows[0],
    );

    frame.render_widget(
        Paragraph::new(
            Line::from(Span::styled(
                "Local data only · zero telemetry · zero network",
                label,
            ))
            .centered(),
        ),
        rows[1],
    );

    frame.render_widget(
        footer(theme, &["Esc / q: back", "Ctrl+T: theme", "Ctrl+X: reset"]),
        rows[2],
    );
}
