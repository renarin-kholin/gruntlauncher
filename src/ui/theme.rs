use iced::{Theme, color, theme::Palette};

const GRUNT_PALETTE_DARK: Palette = Palette {
    background: color!(0x1d1816),
    primary: color!(0xc08459),
    text: color!(0xf3f0ed),
    success: color!(0x839446),
    warning: color!(0xc09c46),
    danger: color!(0x945646),
};

pub fn grunt_theme() -> Theme {
    Theme::custom("GruntTheme", GRUNT_PALETTE_DARK)
}
