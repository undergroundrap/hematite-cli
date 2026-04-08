use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

pub struct ActiveReview {
    pub worker_id: String,
    pub file_path: String,
    pub before: String,
    pub after: String,
    pub tx: tokio::sync::oneshot::Sender<crate::agent::swarm::ReviewResponse>,
}

pub fn draw_diff_review(f: &mut Frame, review: &ActiveReview) {
    let area = centered_rect(80, 80, f.size());
    f.render_widget(Clear, area);

    // ── Outer Frame ──────────────────────────────────────────────
    let rust_color = Color::Rgb(120, 70, 50);
    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(rust_color))
        .style(Style::default().bg(Color::Rgb(15, 10, 5))); // Deep soil background

    let inner_area = outer_block.inner(area);
    f.render_widget(outer_block, area);

    // ── Inner Layout (Hardened for connectivity) ───────────────────────────
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1) // Vital padding to prevent sub-box bleed
        .constraints(
            [
                Constraint::Length(3),      // Header
                Constraint::Percentage(40), // Original
                Constraint::Percentage(40), // Synthesized
                Constraint::Length(3),      // Footer
            ]
            .as_ref(),
        )
        .split(inner_area);

    let title = if review.worker_id.is_empty() {
        "⚠  SYNTHESIS MERGE CONFLICT  ⚠".to_string()
    } else {
        format!("⚙  SWARM SYNTHESIS: Worker {}  ⚙", review.worker_id)
    };

    let header = Paragraph::new(title)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::Yellow)),
        )
        .alignment(ratatui::layout::Alignment::Center);
    f.render_widget(header, chunks[0]);

    // Draw Before (Red Trace)
    let before_para = Paragraph::new(review.before.clone())
        .block(
            Block::default()
                .title(" Original Context ")
                .borders(Borders::ALL)
                .fg(Color::Red),
        )
        .wrap(Wrap { trim: true });
    f.render_widget(before_para, chunks[1]);

    // Draw After (Cyan diff trace)
    // Automatically strip AI-reasoning/thought noise for a clean professional view
    let cleaned_after = review
        .after
        .replace("<thought>", "")
        .replace("</thought>", "")
        .replace("<think>", "")
        .replace("</think>", "");
    let final_after = if let Some(start) = cleaned_after.find("```") {
        let rest = &cleaned_after[start + 3..];
        if let Some(end) = rest.find("```") {
            // Find the first newline to skip language identifier if present
            if let Some(first_line) = rest.find('\n') {
                if first_line < end {
                    rest[first_line + 1..end].trim().to_string()
                } else {
                    rest[..end].trim().to_string()
                }
            } else {
                rest[..end].trim().to_string()
            }
        } else {
            cleaned_after
        }
    } else {
        cleaned_after
    };

    let after_para = Paragraph::new(final_after)
        .block(
            Block::default()
                .title(" AI Synthesized Patch ")
                .borders(Borders::ALL)
                .fg(Color::Cyan),
        )
        .wrap(Wrap { trim: true });
    f.render_widget(after_para, chunks[2]);

    // Footer Triggers
    let footer = Paragraph::new(" [Y] Accept  |  [N] Abort  |  [R] Retry Synthesis ")
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .alignment(ratatui::layout::Alignment::Center);
    f.render_widget(footer, chunks[3]);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}
