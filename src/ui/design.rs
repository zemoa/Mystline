use iced::widget::{
    button as button_widget, checkbox as checkbox_widget, container,
    scrollable as scrollable_widget, text_input,
};
use iced::{Background, Border, Color, Shadow, Theme, Vector, theme};
use std::sync::LazyLock;

pub const BACKGROUND: Color = Color::from_rgb8(0xF4, 0xF7, 0xFB);
pub const SURFACE: Color = Color::WHITE;
pub const TEXT: Color = Color::from_rgb8(0x0F, 0x17, 0x2A);
pub const MUTED: Color = Color::from_rgb8(0x64, 0x74, 0x8B);
pub const BORDER: Color = Color::from_rgb8(0xDC, 0xE3, 0xED);
pub const PRIMARY: Color = Color::from_rgb8(0x08, 0x66, 0xF5);
pub const PRIMARY_HOVER: Color = Color::from_rgb8(0x05, 0x55, 0xD9);
pub const PRIMARY_SOFT: Color = Color::from_rgb8(0xEA, 0xF2, 0xFF);
pub const DANGER: Color = Color::from_rgb8(0xB4, 0x2D, 0x2D);
pub const DANGER_SOFT: Color = Color::from_rgb8(0xFE, 0xF2, 0xF2);
pub const WARNING: Color = Color::from_rgb8(0xA0, 0x5F, 0x0F);

pub const SPACE_1: f32 = 8.0;
pub const SPACE_2: f32 = 12.0;
pub const SPACE_3: f32 = 16.0;
pub const SPACE_4: f32 = 20.0;
pub const RADIUS_SMALL: f32 = 8.0;
pub const RADIUS_LARGE: f32 = 12.0;

static THEME: LazyLock<Theme> = LazyLock::new(|| {
    Theme::custom(
        "Mystline",
        theme::Palette {
            background: BACKGROUND,
            text: TEXT,
            primary: PRIMARY,
            success: Color::from_rgb8(0x0F, 0x8A, 0x6A),
            warning: WARNING,
            danger: DANGER,
        },
    )
});

pub fn theme() -> Theme {
    THEME.clone()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonKind {
    Primary,
    Secondary,
    Ghost,
    Tag(usize),
}

pub fn button(kind: ButtonKind, status: button_widget::Status) -> button_widget::Style {
    let hovered = matches!(
        status,
        button_widget::Status::Hovered | button_widget::Status::Pressed
    );
    let disabled = matches!(status, button_widget::Status::Disabled);
    let (background, text_color, border) = match kind {
        ButtonKind::Primary => (
            Some(Background::Color(if hovered {
                PRIMARY_HOVER
            } else {
                PRIMARY
            })),
            Color::WHITE,
            Border::default().rounded(RADIUS_SMALL),
        ),
        ButtonKind::Secondary => (
            Some(Background::Color(if hovered {
                PRIMARY_SOFT
            } else {
                SURFACE
            })),
            if hovered { PRIMARY } else { TEXT },
            Border {
                color: if hovered { PRIMARY } else { BORDER },
                width: 1.0,
                radius: RADIUS_SMALL.into(),
            },
        ),
        ButtonKind::Ghost => (
            hovered.then_some(Background::Color(PRIMARY_SOFT)),
            if hovered { PRIMARY } else { MUTED },
            Border::default().rounded(RADIUS_SMALL),
        ),
        ButtonKind::Tag(tone) => {
            let (background, foreground) = tag_colors(tone);
            (
                Some(Background::Color(if hovered {
                    mix(background, PRIMARY, 0.08)
                } else {
                    background
                })),
                foreground,
                Border::default().rounded(20.0),
            )
        }
    };
    button_widget::Style {
        background,
        text_color: if disabled {
            Color {
                a: 0.45,
                ..text_color
            }
        } else {
            text_color
        },
        border,
        shadow: if kind == ButtonKind::Primary && !disabled {
            Shadow {
                color: Color::from_rgba8(0x08, 0x66, 0xF5, 0.18),
                offset: Vector::new(0.0, 2.0),
                blur_radius: 6.0,
            }
        } else {
            Shadow::default()
        },
        ..button_widget::Style::default()
    }
}

pub fn input(_theme: &Theme, status: text_input::Status) -> text_input::Style {
    let focused = matches!(status, text_input::Status::Focused { .. });
    text_input::Style {
        background: Background::Color(SURFACE),
        border: Border {
            color: if focused { PRIMARY } else { BORDER },
            width: if focused { 2.0 } else { 1.0 },
            radius: RADIUS_SMALL.into(),
        },
        icon: MUTED,
        placeholder: Color::from_rgb8(0x94, 0xA3, 0xB8),
        value: TEXT,
        selection: Color::from_rgb8(0xBF, 0xDB, 0xFE),
    }
}

pub fn checkbox(_theme: &Theme, status: checkbox_widget::Status) -> checkbox_widget::Style {
    let (checked, hovered) = match status {
        checkbox_widget::Status::Active { is_checked } => (is_checked, false),
        checkbox_widget::Status::Hovered { is_checked } => (is_checked, true),
        checkbox_widget::Status::Disabled { is_checked } => (is_checked, false),
    };
    checkbox_widget::Style {
        background: Background::Color(if checked { PRIMARY } else { SURFACE }),
        icon_color: Color::WHITE,
        border: Border {
            color: if checked || hovered { PRIMARY } else { MUTED },
            width: if hovered { 2.0 } else { 1.5 },
            radius: 5.0.into(),
        },
        text_color: Some(TEXT),
    }
}

pub fn card(_theme: &Theme) -> container::Style {
    container::Style::default()
        .background(SURFACE)
        .color(TEXT)
        .border(Border::default().rounded(RADIUS_LARGE))
}

pub fn panel(_theme: &Theme) -> container::Style {
    container::Style::default()
        .background(SURFACE)
        .color(TEXT)
        .border(Border {
            color: BORDER,
            width: 1.0,
            radius: RADIUS_LARGE.into(),
        })
}

pub fn selected(_theme: &Theme) -> container::Style {
    container::Style::default()
        .background(PRIMARY_SOFT)
        .border(Border {
            color: PRIMARY,
            width: 1.0,
            radius: RADIUS_SMALL.into(),
        })
}

pub fn callout_danger(_theme: &Theme) -> container::Style {
    container::Style::default()
        .background(DANGER_SOFT)
        .color(DANGER)
        .border(Border {
            color: Color::from_rgb8(0xFE, 0xCA, 0xCA),
            width: 1.0,
            radius: RADIUS_SMALL.into(),
        })
}

pub fn callout_help(_theme: &Theme) -> container::Style {
    container::Style::default()
        .background(PRIMARY_SOFT)
        .color(TEXT)
        .border(Border {
            color: Color::from_rgb8(0xBF, 0xDB, 0xFE),
            width: 1.0,
            radius: RADIUS_SMALL.into(),
        })
}

pub fn section_rule(_theme: &Theme) -> container::Style {
    container::Style::default().background(BORDER)
}

pub fn scrollable(theme: &Theme, status: scrollable_widget::Status) -> scrollable_widget::Style {
    let active = matches!(status, scrollable_widget::Status::Active { .. });
    let mut style = scrollable_widget::default(theme, status);
    style.vertical_rail.background = None;
    style.vertical_rail.scroller.background =
        Background::Color(if active { BORDER } else { PRIMARY });
    style.horizontal_rail.background = None;
    style.horizontal_rail.scroller.background = style.vertical_rail.scroller.background;
    style
}

pub fn selection_rail(_theme: &Theme) -> container::Style {
    container::Style::default()
        .background(PRIMARY)
        .border(Border::default().rounded(2.0))
}

pub fn tag_tone(tag: &str) -> usize {
    tag.to_lowercase().bytes().fold(0usize, |hash, byte| {
        hash.wrapping_mul(31).wrapping_add(byte as usize)
    }) % 4
}

fn tag_colors(tone: usize) -> (Color, Color) {
    match tone % 4 {
        0 => (
            Color::from_rgb8(0xDB, 0xEA, 0xFE),
            Color::from_rgb8(0x1D, 0x4E, 0xD8),
        ),
        1 => (
            Color::from_rgb8(0xD1, 0xFA, 0xE5),
            Color::from_rgb8(0x04, 0x78, 0x57),
        ),
        2 => (
            Color::from_rgb8(0xED, 0xE9, 0xFE),
            Color::from_rgb8(0x6D, 0x28, 0xD9),
        ),
        _ => (
            Color::from_rgb8(0xE2, 0xE8, 0xF0),
            Color::from_rgb8(0x47, 0x55, 0x69),
        ),
    }
}

fn mix(left: Color, right: Color, amount: f32) -> Color {
    Color {
        r: left.r + (right.r - left.r) * amount,
        g: left.g + (right.g - left.g) * amount,
        b: left.b + (right.b - left.b) * amount,
        a: left.a + (right.a - left.a) * amount,
    }
}
