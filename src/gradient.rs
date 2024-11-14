use colored::*;

pub struct GradientText;

#[allow(dead_code)]
impl GradientText {
    pub fn info(text: &str) -> String {
        text.bright_blue().bold().to_string()
    }

    pub fn success(text: &str) -> String {
        text.bright_green().bold().to_string()
    }

    pub fn warning(text: &str) -> String {
        text.bright_yellow().bold().to_string()
    }

    pub fn error(text: &str) -> String {
        text.bright_red().bold().to_string()
    }

    pub fn cyber(text: &str) -> String {
        let colors = [
            "38;2;0;255;255",
            "38;2;255;0;255", // Magenta
            "38;2;0;255;127", // Spring Green
        ];
        Self::gradient_text(text, &colors)
    }

    pub fn rainbow(text: &str) -> String {
        let colors = [
            "38;2;255;0;0",   // Red
            "38;2;255;127;0", // Orange
            "38;2;255;255;0", // Yellow
            "38;2;0;255;0",   // Green
            "38;2;0;0;255",   // Blue
            "38;2;139;0;255", // Violet
        ];
        Self::gradient_text(text, &colors)
    }

    pub fn status(text: &str) -> String {
        let colors = [
            "38;2;100;149;237", // Cornflower Blue
            "38;2;0;191;255",   // Deep Sky Blue
        ];
        Self::gradient_text(text, &colors)
    }

    fn gradient_text(text: &str, colors: &[&str]) -> String {
        let mut colored_text = String::new();
        for (i, c) in text.chars().enumerate() {
            let color_index = i % colors.len();
            colored_text.push_str(&format!("\x1b[{}m{}", colors[color_index], c));
        }
        colored_text.push_str("\x1b[0m");
        colored_text
    }
}
