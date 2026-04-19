//! Output formatting utilities: text tables, JSON serialization, color control.

use owo_colors::{DynColors, OwoColorize};
use std::sync::atomic::{AtomicBool, Ordering};

static USE_COLOR: AtomicBool = AtomicBool::new(false);

/// Enable or disable colored output globally.
pub fn set_use_color(enabled: bool) {
    USE_COLOR.store(enabled, Ordering::SeqCst);
}

/// Check if colored output is enabled.
pub fn should_use_color() -> bool {
    USE_COLOR.load(Ordering::SeqCst)
}

/// Colored string builder for conditionally-colored output.
pub struct ColoredString {
    text: String,
    color: Option<DynColors>,
}

impl ColoredString {
    #[expect(dead_code, reason = "surface wired for future flows — see WP32 follow-up")]
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            color: None,
        }
    }

    #[expect(dead_code, reason = "surface wired for future flows — see WP32 follow-up")]
    pub fn with_color(mut self, color: DynColors) -> Self {
        self.color = Some(color);
        self
    }

    fn render(&self) -> String {
        if should_use_color() {
            if let Some(color) = self.color {
                format!("{}", self.text.color(color))
            } else {
                self.text.clone()
            }
        } else {
            self.text.clone()
        }
    }
}

impl std::fmt::Display for ColoredString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.render())
    }
}

/// Format bytes as human-readable string (e.g., "2.5 GB").
pub fn format_bytes(bytes: u64) -> String {
    humansize::format_size(bytes, humansize::DECIMAL)
}

/// Format percentage with one decimal place.
pub fn format_percent(value: f32) -> String {
    format!("{:.1}%", value)
}

/// Format temperature in Celsius. Used by telemetry-rendering flows.
pub fn format_temp(celsius: f32) -> String {
    format!("{:.1}°C", celsius)
}

/// Format power in watts. Used by telemetry-rendering flows.
pub fn format_power(watts: f32) -> String {
    format!("{:.1}W", watts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        // humansize::DECIMAL uses 1 kB = 1000 bytes (SI units).
        assert_eq!(format_bytes(1_000), "1 kB");
        assert_eq!(format_bytes(1_000_000), "1 MB");
        assert_eq!(format_bytes(1_000_000_000), "1 GB");
    }

    #[test]
    fn test_format_percent() {
        assert_eq!(format_percent(50.5), "50.5%");
        assert_eq!(format_percent(100.0), "100.0%");
    }

    #[test]
    fn test_format_temp() {
        assert_eq!(format_temp(45.5), "45.5°C");
    }

    #[test]
    fn test_format_power() {
        assert_eq!(format_power(250.5), "250.5W");
    }

    #[test]
    fn test_color_output_respects_flag() {
        set_use_color(false);
        assert!(!should_use_color());
        set_use_color(true);
        assert!(should_use_color());
    }
}
