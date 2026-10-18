use std::sync::LazyLock;

use iced::{
    Color, Theme,
    theme::{Palette, palette},
};
use serde::Deserialize;

use crate::core::config::ThemeChoice;

// Embed presets in the executable: release archives/installers need no theme files.
static DEFAULT: LazyLock<Theme> = LazyLock::new(|| {
    parse_theme(
        ThemeChoice::Default,
        include_str!("../../theme/default.toml"),
    )
});
static LIGHT_HIGH_CONTRAST: LazyLock<Theme> = LazyLock::new(|| {
    parse_theme(
        ThemeChoice::LightHighContrast,
        include_str!("../../theme/light-high-contrast.toml"),
    )
});
static DARK_HIGH_CONTRAST: LazyLock<Theme> = LazyLock::new(|| {
    parse_theme(
        ThemeChoice::DarkHighContrast,
        include_str!("../../theme/dark-high-contrast.toml"),
    )
});

#[derive(Clone, Copy, Deserialize)]
#[serde(try_from = "String")]
struct HexColor(Color);

impl TryFrom<String> for HexColor {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let hex = value
            .strip_prefix('#')
            .filter(|hex| hex.len() == 6 && hex.bytes().all(|b| b.is_ascii_hexdigit()))
            .ok_or("expected a color in #RRGGBB format")?;
        let rgb = u32::from_str_radix(hex, 16).map_err(|_| "invalid hex color")?;
        Ok(Self(Color::from_rgb8(
            (rgb >> 16) as u8,
            (rgb >> 8) as u8,
            rgb as u8,
        )))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ThemeDefinition {
    background: HexColor,
    primary: HexColor,
    text: HexColor,
    success: HexColor,
    warning: HexColor,
    danger: HexColor,
    #[serde(default)]
    border: Option<HexColor>,
}

fn parse_theme(choice: ThemeChoice, source: &str) -> Theme {
    let definition: ThemeDefinition = toml::from_str(source).expect("invalid bundled theme");
    Theme::custom_with_fn(
        choice.to_string(),
        Palette {
            background: definition.background.0,
            primary: definition.primary.0,
            text: definition.text.0,
            success: definition.success.0,
            warning: definition.warning.0,
            danger: definition.danger.0,
        },
        |palette| {
            let mut extended = palette::Extended::generate(palette);
            if let Some(border) = definition.border {
                // Iced derives these border/separator shades from the background.
                // Near pure black they are practically invisible; allow an explicit
                // shade while keeping the base text/background and accents intact.
                let pair = palette::Pair {
                    color: border.0,
                    text: palette.text,
                };
                extended.background.weak = pair;
                extended.background.neutral = pair;
                extended.background.strong = pair;
                extended.background.stronger = pair;
                extended.background.strongest = pair;
            }
            extended
        },
    )
}

pub fn grunt_theme(choice: ThemeChoice) -> Theme {
    match choice {
        ThemeChoice::Default => DEFAULT.clone(),
        ThemeChoice::LightHighContrast => LIGHT_HIGH_CONTRAST.clone(),
        ThemeChoice::DarkHighContrast => DARK_HIGH_CONTRAST.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::{
        Background, color,
        widget::{button, container, rule, text_input},
    };

    #[test]
    fn default_matches_upstream_palette_and_generated_shades() {
        let original = Theme::custom(
            "GruntTheme",
            Palette {
                background: color!(0x1d1816),
                primary: color!(0xc08459),
                text: color!(0xf3f0ed),
                success: color!(0x839446),
                warning: color!(0xc09c46),
                danger: color!(0x945646),
            },
        );
        let bundled = grunt_theme(ThemeChoice::Default);
        assert_eq!(bundled.palette(), original.palette());
        assert_eq!(bundled.extended_palette(), original.extended_palette());
        let dark = grunt_theme(ThemeChoice::DarkHighContrast);
        assert_eq!(
            dark.palette(),
            Palette {
                background: Color::BLACK,
                text: Color::WHITE,
                ..original.palette()
            }
        );
    }

    #[test]
    fn light_theme_has_black_on_white_and_readable_enabled_buttons() {
        let theme = grunt_theme(ThemeChoice::LightHighContrast);
        assert_eq!(theme.palette().text, Color::BLACK);
        assert_eq!(theme.palette().background, Color::WHITE);
        for style in [
            button::primary,
            button::secondary,
            button::success,
            button::warning,
            button::danger,
            button::subtle,
        ] {
            for status in [
                button::Status::Active,
                button::Status::Hovered,
                button::Status::Pressed,
            ] {
                let style = style(&theme, status);
                let Some(Background::Color(background)) = style.background else {
                    panic!("expected solid button fill")
                };
                assert!(
                    style.text_color.relative_contrast(background) >= 4.5,
                    "{style:?}"
                );
            }
        }
    }

    #[test]
    fn palette_colors_require_six_hex_digits() {
        for invalid in ["red", "#FFF", "#GGGGGG", "#é0000"] {
            assert!(HexColor::try_from(invalid.to_string()).is_err());
        }
    }

    #[test]
    fn dark_theme_has_visible_panel_input_and_separator_borders() {
        let theme = grunt_theme(ThemeChoice::DarkHighContrast);
        let panel = container::bordered_box(&theme);
        let input = text_input::default(&theme, text_input::Status::Active);
        let Some(Background::Color(panel_background)) = panel.background else {
            panic!("expected a solid panel background")
        };
        for (border, background) in [
            (panel.border.color, panel_background),
            (input.border.color, theme.palette().background),
            (rule::default(&theme).color, panel_background),
        ] {
            assert!(
                border.relative_contrast(background) >= 3.0,
                "border {border:?} is too close to background {background:?}"
            );
        }
    }
}
