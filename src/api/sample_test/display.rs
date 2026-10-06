use crossterm::style::Stylize as _;
use unicode_width::{UnicodeWidthChar as _, UnicodeWidthStr as _};

pub struct Window {
    title: String,
    content: String,
}

impl Window {
    pub fn new(title: impl ToString, content: impl ToString) -> Self {
        Self {
            title: title.to_string(),
            content: content.to_string(),
        }
    }
    fn content_lines(&self, width: usize) -> Vec<String> {
        let mut content_lines = Vec::new();
        for line in self.content.lines() {
            let mut temp_line = String::new();
            for c in line.chars() {
                if temp_line.width() + c.width().unwrap_or(0) > width.saturating_sub(4) {
                    content_lines.push(std::mem::take(&mut temp_line));
                }
                temp_line.push(c);
            }
            if !temp_line.is_empty() {
                content_lines.push(temp_line);
            }
        }
        content_lines
    }
    fn window_lines(
        title: &str,
        content_lines: &[String],
        height: usize,
        width: usize,
    ) -> Vec<String> {
        let mut window_lines = Vec::with_capacity(height);
        let vertical = '│'.dark_grey();
        window_lines.push(
            format!(
                "╭ {} {}╮",
                title,
                "─".repeat(width.saturating_sub(title.width() + 4))
            )
            .dark_grey()
            .to_string(),
        );
        for content_line in content_lines
            .iter()
            .chain(std::iter::repeat(&String::new()))
            .take(height.saturating_sub(2))
        {
            window_lines.push(format!(
                "{vertical} {content_line}{} {vertical}",
                " ".repeat(width.saturating_sub(content_line.width() + 4))
            ));
        }
        window_lines.push(
            format!("╰{}╯", "─".repeat(width.saturating_sub(2)))
                .dark_grey()
                .to_string(),
        );
        window_lines
    }
    pub fn print(&self, width: usize) {
        let content_lines = self.content_lines(width);
        let window_lines =
            Self::window_lines(&self.title, &content_lines, content_lines.len() + 2, width);
        for line in window_lines {
            eprintln!("{line}");
        }
    }
    pub fn horizontal_print(windows: &[Self], width: usize) {
        if windows.is_empty() {
            return;
        }
        let single_width = (width + 1) / windows.len() - 1;
        let lines: Vec<_> = windows
            .iter()
            .map(|window| (&window.title, window.content_lines(single_width)))
            .collect();
        let height = lines.iter().map(|l| l.1.len()).max().unwrap() + 2;
        let window_lines: Vec<_> = lines
            .iter()
            .map(|l| Self::window_lines(l.0, &l.1, height, single_width))
            .collect();
        for i in 0..height {
            eprint!("{}", window_lines[0][i]);
            for lines in &window_lines[1..] {
                eprint!(" {}", lines[i]);
            }
            eprintln!()
        }
    }
}
