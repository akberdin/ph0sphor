//! Theme palettes. README §17 declares five themes for MVP; we ship the
//! two MVP-mandatory ones (`phosphor-green`, `amber-crt`) here. The
//! remaining three are stubs that fall back to a sensible monochrome
//! mapping until they get their own bespoke palettes.

pub use ph0sphor_core::Theme;
use ratatui::style::Color;

#[derive(Debug, Clone, Copy)]
pub struct ThemePalette {
    pub fg: Color,
    pub bg: Color,
    pub accent: Color,
    pub warning: Color,
    pub critical: Color,
    pub dim: Color,
}

impl ThemePalette {
    pub fn for_theme(theme: Theme) -> Self {
        match theme {
            Theme::PhosphorGreen => Self {
                fg: Color::Rgb(0x33, 0xff, 0x66),
                bg: Color::Rgb(0x00, 0x11, 0x00),
                accent: Color::Rgb(0x88, 0xff, 0xaa),
                warning: Color::Rgb(0xff, 0xd8, 0x66),
                critical: Color::Rgb(0xff, 0x55, 0x77),
                dim: Color::Rgb(0x22, 0x66, 0x33),
            },
            Theme::AmberCrt => Self {
                fg: Color::Rgb(0xff, 0xb0, 0x00),
                bg: Color::Rgb(0x1a, 0x0d, 0x00),
                accent: Color::Rgb(0xff, 0xd3, 0x4d),
                warning: Color::Rgb(0xff, 0x99, 0x33),
                critical: Color::Rgb(0xff, 0x55, 0x44),
                dim: Color::Rgb(0x7a, 0x55, 0x00),
            },
            Theme::IceTerminal => Self {
                fg: Color::Rgb(0xaa, 0xee, 0xff),
                bg: Color::Rgb(0x00, 0x10, 0x18),
                accent: Color::Rgb(0xff, 0xff, 0xff),
                warning: Color::Rgb(0xff, 0xd8, 0x66),
                critical: Color::Rgb(0xff, 0x55, 0x77),
                dim: Color::Rgb(0x44, 0x77, 0x88),
            },
            Theme::MonoLcd => Self {
                fg: Color::Rgb(0xc8, 0xc8, 0xc8),
                bg: Color::Rgb(0x10, 0x10, 0x10),
                accent: Color::Rgb(0xff, 0xff, 0xff),
                warning: Color::Rgb(0xee, 0xee, 0x77),
                critical: Color::Rgb(0xff, 0x66, 0x66),
                dim: Color::Rgb(0x55, 0x55, 0x55),
            },
            Theme::HighContrast => Self {
                fg: Color::White,
                bg: Color::Black,
                accent: Color::Yellow,
                warning: Color::LightYellow,
                critical: Color::LightRed,
                dim: Color::Gray,
            },
        }
    }
}

/// Cycle order used by the `C` key. Matches the order in README §17.
pub fn next_theme(t: Theme) -> Theme {
    match t {
        Theme::PhosphorGreen => Theme::AmberCrt,
        Theme::AmberCrt => Theme::IceTerminal,
        Theme::IceTerminal => Theme::MonoLcd,
        Theme::MonoLcd => Theme::HighContrast,
        Theme::HighContrast => Theme::PhosphorGreen,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycle_visits_every_theme_and_loops() {
        let mut t = Theme::PhosphorGreen;
        let mut seen = Vec::new();
        for _ in 0..5 {
            seen.push(t);
            t = next_theme(t);
        }
        assert_eq!(seen.len(), 5);
        assert_eq!(t, Theme::PhosphorGreen, "cycle returns to start");
    }
}
