use crate::{
    button::{ButtonStateStyle, ButtonStyle},
    shapes::LineWidth,
    wgpu_utils::Srgb,
};

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    /// Primary, secondary and tertiary (in order) background colors.
    pub background: [Srgb; 3],
    /// Primary, secondary and tertiary (in order) foreground colors.
    pub foreground: [Srgb; 3],
    /// Button styles, indexed by `ButtonKind`.
    pub button_styles: [ButtonStyle; 3],
}

impl Theme {
    pub const fn primary_background(&self) -> Srgb {
        self.background[0]
    }

    pub const fn secondary_background(&self) -> Srgb {
        self.background[1]
    }

    pub const fn tertiary_background(&self) -> Srgb {
        self.background[2]
    }

    pub const fn primary_foreground(&self) -> Srgb {
        self.foreground[0]
    }

    pub const fn secondary_foreground(&self) -> Srgb {
        self.foreground[1]
    }

    pub const fn tertiary_foreground(&self) -> Srgb {
        self.foreground[2]
    }

    pub const fn button_style(&self, kind: ButtonKind) -> ButtonStyle {
        self.button_styles[kind.to_usize()]
    }

    pub const DEFAULT: Self = Self {
        background: [
            Srgb::from_hex(0x181818),
            Srgb::from_hex(0x2A2A2A),
            Srgb::from_hex(0x424242),
        ],
        foreground: [
            Srgb::from_hex(0xFFFFFF),
            Srgb::from_hex(0xA2A2A2),
            Srgb::from_hex(0x494949),
        ],
        button_styles: [
            // Normal.
            ButtonStyle {
                // Idle.
                idle_style: ButtonStateStyle {
                    line_width: LineWidth::Uniform(2.),
                    font_size: 12.,
                    text_color: Srgb::from_hex(0xFFFFFF),
                    fill_color: Srgb::from_hex(0x2A2A2A),
                    line_color: Srgb::from_hex(0x494949),
                },
                // Hovered.
                hovered_style: ButtonStateStyle {
                    line_width: LineWidth::Uniform(2.),
                    font_size: 12.,
                    text_color: Srgb::from_hex(0xFFFFFF),
                    fill_color: Srgb::from_hex(0x424242),
                    line_color: Srgb::from_hex(0xA2A2A2),
                },
                // Pressed.
                pressed_style: ButtonStateStyle {
                    line_width: LineWidth::Uniform(2.),
                    font_size: 12.,
                    text_color: Srgb::from_hex(0xFFFFFF),
                    fill_color: Srgb::from_hex(0xA2A2A2),
                    line_color: Srgb::from_hex(0xA2A2A2),
                },
            },
            // Primary.
            ButtonStyle {
                // Idle.
                idle_style: ButtonStateStyle {
                    line_width: LineWidth::Uniform(2.),
                    font_size: 12.,
                    text_color: Srgb::from_hex(0xFFFFFF),
                    fill_color: Srgb::from_hex(0x2C3F71),
                    line_color: Srgb::from_hex(0x3D5B9B),
                },
                // Hovered.
                hovered_style: ButtonStateStyle {
                    line_width: LineWidth::Uniform(2.),
                    font_size: 12.,
                    text_color: Srgb::from_hex(0xFFFFFF),
                    fill_color: Srgb::from_hex(0x5771B2),
                    line_color: Srgb::from_hex(0x95A0BD),
                },
                // Pressed.
                pressed_style: ButtonStateStyle {
                    line_width: LineWidth::Uniform(2.),
                    font_size: 12.,
                    text_color: Srgb::from_hex(0xFFFFFF),
                    fill_color: Srgb::from_hex(0x95A0BD),
                    line_color: Srgb::from_hex(0x95A0BD),
                },
            },
            // Toxic.
            ButtonStyle {
                // Idle.
                idle_style: ButtonStateStyle {
                    line_width: LineWidth::Uniform(2.),
                    font_size: 12.,
                    text_color: Srgb::from_hex(0xFFFFFF),
                    fill_color: Srgb::from_hex(0x952727),
                    line_color: Srgb::from_hex(0xC83F3F),
                },
                // Hovered.
                hovered_style: ButtonStateStyle {
                    line_width: LineWidth::Uniform(2.),
                    font_size: 12.,
                    text_color: Srgb::from_hex(0xFFFFFF),
                    fill_color: Srgb::from_hex(0xFF776C),
                    line_color: Srgb::from_hex(0xFFD0CE),
                },
                // Pressed.
                pressed_style: ButtonStateStyle {
                    line_width: LineWidth::Uniform(2.),
                    font_size: 12.,
                    text_color: Srgb::from_hex(0xFFFFFF),
                    fill_color: Srgb::from_hex(0xFFD0CE),
                    line_color: Srgb::from_hex(0xFFD0CE),
                },
            },
        ],
    };
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ButtonKind {
    #[default]
    /// Untinted button.
    Mundane = 0,
    /// Non-alarm color tinted button.
    Primary,
    /// Alarm color (e.g. red) tinted button.
    Toxic,
}

impl ButtonKind {
    pub const fn to_usize(self) -> usize {
        self as u8 as usize
    }
}
