use iced::Color;

/// Parses a six- or eight-digit hex color, with an optional `#` prefix.
#[must_use]
pub fn parse_hex(value: &str) -> Option<Color> {
    let value = value.trim();
    let hex = value.strip_prefix('#').unwrap_or(value);
    if !matches!(hex.len(), 6 | 8) || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    hex.parse().ok()
}

pub fn format_hex(color: Color) -> String {
    let [red, green, blue, alpha] = color.into_rgba8();
    if alpha == u8::MAX {
        return format!("#{red:02X}{green:02X}{blue:02X}");
    }
    format!("#{red:02X}{green:02X}{blue:02X}{alpha:02X}")
}
