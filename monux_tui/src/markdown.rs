use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag};
use ratatui::{
    style::{Color, Modifier, Style},
    text::Span,
};


pub fn render_markdown_segment(segment: &str, base_style: Style) ->
Vec<Span<'static>> {
    let trimmed = segment.trim_start();
    let indent_len = segment.len().saturating_sub(trimmed.len());
    let mut styled_base = base_style;


    if let Some((depth, consumed)) = parse_blockquote_prefix(trimmed)
{
        let _ = consumed;
        styled_base = blockquote_style_for_depth(styled_base, depth);
    } else if trimmed.starts_with("```") {
        styled_base =
styled_base.bg(Color::DarkGray).fg(Color::White);
    }


    let text = &segment[indent_len..];
    render_with_prefix(text, indent_len, segment, styled_base)
}


fn render_with_prefix(
    text: &str,
    indent_len: usize,
    segment: &str,
    styled_base: Style,
) -> Vec<Span<'static>> {
    let chars: Vec<char> = text.chars().collect();
    let mut styles = vec![styled_base; chars.len()];
    let byte_to_char = build_byte_to_char_map(text);


    let options = Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TABLES
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_FOOTNOTES;
    let parser = Parser::new_ext(text, options).into_offset_iter();
    let mut style_stack = vec![styled_base];


    for (event, range) in parser {
        match event {
            Event::Start(tag) => {
                let current =
*style_stack.last().unwrap_or(&styled_base);
                let next = match tag {
                    Tag::Emphasis =>
current.add_modifier(Modifier::ITALIC),
                    Tag::Strong =>
current.add_modifier(Modifier::BOLD),
                    Tag::Strikethrough =>
current.add_modifier(Modifier::CROSSED_OUT),
                    Tag::CodeBlock(_) =>
current.bg(Color::DarkGray).fg(Color::White),
                    Tag::Link { .. } =>
current.fg(Color::Green).add_modifier(Modifier::UNDERLINED),
                    Tag::Image { .. } =>
current.fg(Color::Yellow).add_modifier(Modifier::BOLD),
                    Tag::Heading { level, .. } =>
heading_style_for_level(current, level),
                    _ => current,
                };
                style_stack.push(next);
            }
            Event::End(_) => {
                if style_stack.len() > 1 {
                    style_stack.pop();
                }
            }
            Event::Code(_) => {
                let style = style_stack
                    .last()
                    .copied()
                    .unwrap_or(styled_base)
                    .bg(Color::DarkGray)
                    .fg(Color::Yellow);
                apply_style_for_byte_range(
                    &mut styles,
                    &byte_to_char,
                    range.start,
                    range.end,
                    style,
                );
            }
            Event::Text(_) | Event::Html(_) | Event::InlineHtml(_) =>
{
                let style =
*style_stack.last().unwrap_or(&styled_base);
                apply_style_for_byte_range(
                    &mut styles,
                    &byte_to_char,
                    range.start,
                    range.end,
                    style,
                );
            }
            Event::TaskListMarker(checked) => {
                let mut style = style_stack
                    .last()
                    .copied()
                    .unwrap_or(styled_base)
                    .fg(if checked { Color::Green } else {
Color::Yellow });
                if checked {
                    style = style.add_modifier(Modifier::CROSSED_OUT);
                } else {
                    style = style.add_modifier(Modifier::DIM);
                }
                apply_style_for_byte_range(
                    &mut styles,
                    &byte_to_char,
                    range.start,
                    range.end,
                    style,
                );
            }
            Event::FootnoteReference(_) => {
                let style = style_stack
                    .last()
                    .copied()
                    .unwrap_or(styled_base)
                    .fg(Color::Blue);
                apply_style_for_byte_range(
                    &mut styles,
                    &byte_to_char,
                    range.start,
                    range.end,
                    style,
                );
            }
            _ => {}
        }
    }


    highlight_markers(text, &byte_to_char, &mut styles, styled_base);
    highlight_list_prefix(text, &byte_to_char, &mut styles,
styled_base);


    highlight_wikilinks(text, &byte_to_char, &mut styles,
styled_base);


    let mut out: Vec<Span<'static>> = Vec::new();
    if indent_len > 0 {
        out.push(Span::styled(segment[..indent_len].to_string(),
styled_base));
    }
    out.extend(spans_from_chars_with_styles(&chars, &styles));
    if out.is_empty() {
        out.push(Span::styled(String::new(), styled_base));
    }
    out
}


fn parse_blockquote_prefix(text: &str) -> Option<(usize, usize)> {
    let mut depth = 0usize;
    let mut i = 0usize;


    while i < text.len() {
        let rest = &text[i..];
        if !rest.starts_with('>') {
            break;
        }
        depth += 1;
        i += 1;


        if i < text.len() && text.as_bytes()[i] == b' ' {
            i += 1;
        }
    }


    if depth == 0 { None } else { Some((depth, i)) }
}


fn blockquote_style_for_depth(base: Style, depth: usize) -> Style {
    match depth {
        1 => base.fg(Color::DarkGray),
        2 => base.fg(Color::Gray),
        _ => base.fg(Color::LightBlue).add_modifier(Modifier::DIM),
    }
}


fn build_byte_to_char_map(text: &str) -> Vec<usize> {
    let mut map = vec![0usize; text.len() + 1];
    let mut char_idx = 0usize;
    for (byte_idx, ch) in text.char_indices() {
        map[byte_idx] = char_idx;
        let next = byte_idx + ch.len_utf8();
        for item in map.iter_mut().take(next + 1).skip(byte_idx + 1) {
            *item = char_idx + 1;
        }
        char_idx += 1;
    }
    map
}


fn apply_style_for_byte_range(
    styles: &mut [Style],
    byte_to_char: &[usize],
    start: usize,
    end: usize,
    style: Style,
) {
    if start >= end || start >= byte_to_char.len() {
        return;
    }
    let s = byte_to_char[start];
    let e = byte_to_char[end.min(byte_to_char.len() - 1)];
    for slot in styles.iter_mut().take(e).skip(s) {
        *slot = slot.patch(style);
    }
}


fn highlight_markers(text: &str, byte_to_char: &[usize], styles: &mut
[Style], base: Style) {
    let marker_style = base.fg(Color::Magenta).add_modifier(Modifier::DIM);
    let bold_marker_style = base.fg(Color::Cyan).add_modifier(Modifier::BOLD);
    let mut i = 0usize;
    while i < text.len() {
        if !text.is_char_boundary(i) {
            i = next_char_boundary(text, i);
            continue;
        }
        let rest = &text[i..];
        let len = if rest.starts_with("```") {
            3
        } else if rest.starts_with("**") || rest.starts_with("__") ||
rest.starts_with("~~") {
            2
        } else if rest.starts_with('`') || rest.starts_with('*') ||
rest.starts_with('_') {
            1
        } else {
            0
        };
        if len > 0 {
            let style = if rest.starts_with("**") || rest.starts_with("__") {
                bold_marker_style
            } else {
                marker_style
            };
            apply_style_for_byte_range(styles, byte_to_char, i, i + len, style);
            i += len;
        } else {
            i = next_char_boundary(text, i);
        }
    }
}


fn highlight_wikilinks(text: &str, byte_to_char: &[usize], styles:
&mut [Style], base: Style) {
    let link_style =
base.fg(Color::Green).add_modifier(Modifier::UNDERLINED);
    let mut i = 0usize;
    while i < text.len() {
        if !text.is_char_boundary(i) {
            i = next_char_boundary(text, i);
            continue;
        }
        let rest = &text[i..];
        if let Some(content) = rest.strip_prefix("[[")
            && let Some(end) = content.find("]]")
        {
            let total = end + 4;
            apply_style_for_byte_range(styles, byte_to_char, i, i +
total, link_style);
            i += total;
            continue;
        }
        i = next_char_boundary(text, i);
    }
}


fn highlight_list_prefix(text: &str, byte_to_char: &[usize], styles:
&mut [Style], base: Style) {
    let mut ws_end = 0usize;
    let mut indent_width = 0usize;
    for (idx, ch) in text.char_indices() {
        if ch == ' ' {
            ws_end = idx + ch.len_utf8();
            indent_width += 1;
            continue;
        }
        if ch == '\t' {
            ws_end = idx + ch.len_utf8();
            indent_width += 4;
            continue;
        }
        break;
    }


    if ws_end > 0 {
        let nested_level = indent_width / 2;
        if nested_level > 0 {
            let indent_style = base.add_modifier(Modifier::DIM);
            apply_style_for_byte_range(styles, byte_to_char, 0,
ws_end, indent_style);
        }
    }


    let rest = &text[ws_end..];
    if rest.is_empty() {
        return;
    }


    let mut marker_len = 0usize;
    let mut marker_is_unordered = false;
    let mut marker_is_ordered = false;
    if let Some(first) = rest.chars().next() {
        if (first == '-' || first == '*' || first == '+') && has_list_separator(rest,
first.len_utf8()) {
            marker_len = first.len_utf8();
            marker_is_unordered = true;
        } else if first.is_ascii_digit() {
            let bytes = rest.as_bytes();
            let mut i = 0usize;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            if i > 0
                && i < bytes.len()
                && (bytes[i] == b'.' || bytes[i] == b')')
                && has_list_separator(rest, i + 1)
            {
                marker_len = i + 1;
                marker_is_ordered = true;
            }
        }
    }

    if marker_len > 0 {
        let marker_end = ws_end + marker_len;
        let marker_sep_end = if marker_end < text.len() {
            marker_end + 1
        } else {
            marker_end
        };
        let marker_style =
base.fg(Color::Cyan).add_modifier(Modifier::BOLD);
        apply_style_for_byte_range(
            styles,
            byte_to_char,
            ws_end,
            marker_end,
            marker_style,
        );

        // Keep list content subtly differentiated from regular paragraph text.
        let content_style = if marker_is_ordered {
            base.add_modifier(Modifier::DIM)
        } else if marker_is_unordered {
            base
        } else {
            base
        };
        apply_style_for_byte_range(
            styles,
            byte_to_char,
            marker_sep_end,
            text.len(),
            content_style,
        );

        // Highlight task list checkbox marker if present.
        let after_marker = &text[marker_sep_end..];
        if after_marker.starts_with("[ ]") || after_marker.starts_with("[x]") || after_marker.starts_with("[X]") {
            let task_style = if after_marker.starts_with("[ ]") {
                base.fg(Color::Yellow).add_modifier(Modifier::DIM)
            } else {
                base.fg(Color::Green).add_modifier(Modifier::BOLD)
            };
            apply_style_for_byte_range(
                styles,
                byte_to_char,
                marker_sep_end,
                marker_sep_end + 3,
                task_style,
            );
        }
    }
}


fn has_list_separator(s: &str, marker_end: usize) -> bool {
    if marker_end >= s.len() {
        return false;
    }
    let sep = s[marker_end..].chars().next();
    matches!(sep, Some(' ' | '\t'))
}


fn next_char_boundary(text: &str, i: usize) -> usize {
    if i >= text.len() {
        return text.len();
    }
    let mut j = (i + 1).min(text.len());
    while j < text.len() && !text.is_char_boundary(j) {
        j += 1;
    }
    j
}


fn spans_from_chars_with_styles(chars: &[char], styles: &[Style]) ->
Vec<Span<'static>> {
    if chars.is_empty() {
        return vec![Span::styled(String::new(), Style::default())];
    }


    let mut out = Vec::new();
    let mut current_style = styles[0];
    let mut buf = String::new();
    buf.push(chars[0]);


    for idx in 1..chars.len() {
        if styles[idx] == current_style {
            buf.push(chars[idx]);
        } else {
            out.push(Span::styled(std::mem::take(&mut buf),
current_style));
            current_style = styles[idx];
            buf.push(chars[idx]);
        }
    }


    out.push(Span::styled(buf, current_style));
    out
}


fn heading_style_for_level(base: Style, level: HeadingLevel) -> Style
{
    match level {
        HeadingLevel::H1 =>
base.add_modifier(Modifier::BOLD).fg(Color::Cyan),
        HeadingLevel::H2 => base.add_modifier(Modifier::BOLD),
        HeadingLevel::H3 => base.add_modifier(Modifier::UNDERLINED),
        HeadingLevel::H4 | HeadingLevel::H5 | HeadingLevel::H6 =>
base.add_modifier(Modifier::DIM),
    }
}
