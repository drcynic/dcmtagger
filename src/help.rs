use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Stylize,
    symbols::border,
    text::Line,
    widgets::{Block, Clear, Padding, Paragraph, Widget},
};

use std::sync::LazyLock;

static HELP_TEXT: LazyLock<Vec<&str>> = LazyLock::new(help_text);

pub fn render_help_overlay(area: Rect, buf: &mut Buffer, scroll_offset: usize) {
    // Calculate centered popup area (roughly 60% width, 70% height)
    let popup_width = (area.width as f32 * 0.6) as u16;
    let popup_height = (area.height as f32 * 0.7) as u16;
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;

    let popup_area = Rect {
        x: area.x + popup_x,
        y: area.y + popup_y,
        width: popup_width,
        height: popup_height,
    };

    Clear.render(popup_area, buf);

    // Get help text lines and handle scrolling
    let help_text = {
        let help_text = &*HELP_TEXT;
        let visible_height = popup_height.saturating_sub(2) as usize; // Account for borders
        let start_line = scroll_offset;
        let end_line = (start_line + visible_height).min(help_text.len());
        help_text[start_line..end_line].join("\n")
    };

    let help_block = Block::bordered()
        .padding(Padding::horizontal(1))
        .title(Line::from(" DICOM Tagger Help".bold()).centered())
        .border_set(border::ROUNDED);

    Paragraph::new(help_text).block(help_block).render(popup_area, buf);
}

pub fn num_help_text_lines() -> usize {
    HELP_TEXT.len()
}

pub fn help_text<'a>() -> Vec<&'a str> {
    raw_help_text().lines().collect::<Vec<&'a str>>()
}

const fn raw_help_text() -> &'static str {
    r#"Navigation:
  q/Esc                - Quit
  1                    - Sort tree by filename
  2                    - Sort tree by tags
  3                    - Sort tree by tags, only showing tags with different values
  i                    - Enter edit mode for selected tag
  /                    - Enter search mode
  ?                    - Show help

  Enter/Space          - Toggle expand/collapse
  j/↓/ctrl+n           - Move down visible tree structure over all hierarchy levels
  k/↑/ctrl+p           - Move up visible tree structure over all hierarchy levels
  h/←                  - Move to parent or close node
  l/→                  - Expand node or move to first child
  H/shift+←            - Move to parent
  L/shift+→            - Move to next child (expand if collapsed)
  J/shift+↓            - Move to next sibling (same level)
  K/shift+↑            - Move to previous sibling (same level)
  g                    - Move to first element
  G                    - Move to last element
  0/^                  - Move to first sibling
  $                    - Move to last sibling
  c                    - Collapse current node and siblings
  e                    - Expand current node and siblings
  E                    - Expand current node recursively
  C                    - Collapse current node recursively

  ctrl+u               - Move half page up
  ctrl+d               - Move half page down
  ctrl+f/page-down     - Move page down
  ctrl+b/page-up       - Move page up

  n                    - Search for next occurence if search text present
  N                    - Search for prev occurence if search text present
"#
}
