/// Renders clean, bold geometric sans-serif numerals (5 rows tall) using Unicode
/// half-blocks (`▀`, `▄`, `█`) for crisp typography in every terminal.
pub fn render_bold_time(mins: u32, secs: u32) -> Vec<String> {
    let text = format!("{:02}:{:02}", mins, secs);
    let mut rows = vec![String::new(); 5];
    for (idx, ch) in text.chars().enumerate() {
        let glyph = glyph_for(ch);
        for r in 0..5 {
            if idx > 0 {
                rows[r].push_str("  ");
            }
            rows[r].push_str(glyph[r]);
        }
    }
    rows
}

fn glyph_for(ch: char) -> [&'static str; 5] {
    match ch {
        '0' => ["▄███▄", "█   █", "█   █", "█   █", "▀███▀"],
        '1' => ["  ▄█ ", " ▀ █ ", "   █ ", "   █ ", " ▄▄█▄"],
        '2' => ["▄███▄", "▀   █", " ▄▄█▀", "█▀   ", "█████"],
        '3' => ["▄███▄", "▀   █", "  ██▄", "▄   █", "▀███▀"],
        '4' => ["   ▄█", " ▄▀ █", "█▄▄▄█", "    █", "    █"],
        '5' => ["█████", "█    ", "████▄", "    █", "▀███▀"],
        '6' => ["▄███▄", "█    ", "████▄", "█   █", "▀███▀"],
        '7' => ["█████", "    █", "   █▀", "  █  ", "  █  "],
        '8' => ["▄███▄", "█   █", "▄███▄", "█   █", "▀███▀"],
        '9' => ["▄███▄", "█   █", "▀████", "    █", "▀███▀"],
        ':' => ["     ", "  ▄  ", "     ", "  ▀  ", "     "],
        _ => ["     ", "     ", "     ", "     ", "     "],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_bold_time_has_five_equal_width_rows() {
        let lines = render_bold_time(25, 0);
        assert_eq!(lines.len(), 5);
        let first_chars = lines[0].chars().count();
        for l in &lines {
            assert_eq!(l.chars().count(), first_chars);
        }
    }
}
