// Hello-world widgets rendered with takumi.
//
// `foo` → a horizontal progress bar (~70% full, blue→green gradient on a
// dark rounded track). Exercises: flex layout, padding, border-radius,
// linear-gradient background.
//
// `bar` → a traffic-light strip with red/yellow/green discs evenly spaced
// in a dark rounded container. Exercises: flex row + space-around,
// multiple children, border-radius producing actual circles.

use std::borrow::Cow;
use std::sync::OnceLock;

use takumi::{
    GlobalContext,
    layout::{
        Viewport,
        node::Node,
        style::{
            Angle, AlignItems, BackgroundImage, Color, ColorInput, Display, FlexDirection,
            GradientStop, JustifyContent, Length, LengthDefaultsToZero, LinearGradient,
            LinearGradientDirection, SpacePair, Style, StyleDeclaration,
        },
    },
    rendering::{ImageOutputFormat, RenderOptions, render, write_image},
};

pub const IMG_W: u32 = 400;
pub const IMG_H: u32 = 200;

const TRACK_BG: [u8; 4] = [42, 45, 58, 255];
const BAR_START: [u8; 4] = [60, 140, 255, 255];
const BAR_END: [u8; 4] = [60, 220, 140, 255];
const LIGHT_RED: [u8; 4] = [240, 70, 70, 255];
const LIGHT_YELLOW: [u8; 4] = [250, 200, 60, 255];
const LIGHT_GREEN: [u8; 4] = [80, 220, 110, 255];

fn context() -> &'static GlobalContext {
    static CTX: OnceLock<GlobalContext> = OnceLock::new();
    CTX.get_or_init(GlobalContext::default)
}

fn bg_color(rgba: [u8; 4]) -> StyleDeclaration {
    StyleDeclaration::background_color(ColorInput::from(Color::from(rgba)))
}

fn radius_all(radius: LengthDefaultsToZero) -> [StyleDeclaration; 4] {
    let pair = SpacePair::from_single(radius);
    [
        StyleDeclaration::border_top_left_radius(pair),
        StyleDeclaration::border_top_right_radius(pair),
        StyleDeclaration::border_bottom_right_radius(pair),
        StyleDeclaration::border_bottom_left_radius(pair),
    ]
}

fn encode_png(image: image::RgbaImage) -> Vec<u8> {
    let mut buf = Vec::with_capacity(8 * 1024);
    write_image(Cow::Owned(image), &mut buf, ImageOutputFormat::Png, None)
        .expect("takumi png encode");
    buf
}

fn render_node(node: Node) -> Vec<u8> {
    let opts = RenderOptions::builder()
        .viewport(Viewport::new((IMG_W, IMG_H)))
        .node(node)
        .global(context())
        .build();
    let img = render(opts).expect("takumi render");
    encode_png(img)
}

pub fn render_progress_bar() -> Vec<u8> {
    let bar_gradient = LinearGradient::builder()
        .direction(LinearGradientDirection::Angle(Angle::new(90.0)))
        .stops([
            GradientStop::ColorHint {
                color: ColorInput::from(Color::from(BAR_START)),
                hint: None,
            },
            GradientStop::ColorHint {
                color: ColorInput::from(Color::from(BAR_END)),
                hint: None,
            },
        ])
        .build();

    let mut bar_style = Style::default()
        .with(StyleDeclaration::width(Length::Percentage(70.0)))
        .with(StyleDeclaration::height(Length::Percentage(100.0)))
        .with(StyleDeclaration::background_image(Some(
            vec![BackgroundImage::Linear(bar_gradient)].into_boxed_slice(),
        )));
    for decl in radius_all(LengthDefaultsToZero::Px(50.0)) {
        bar_style = bar_style.with(decl);
    }
    let bar = Node::container([]).with_style(bar_style);

    let mut track_style = Style::default()
        .with(StyleDeclaration::display(Display::Flex))
        .with(StyleDeclaration::flex_direction(FlexDirection::Row))
        .with(StyleDeclaration::align_items(AlignItems::Center))
        .with(StyleDeclaration::justify_content(JustifyContent::FlexStart))
        .with(StyleDeclaration::width(Length::Px(IMG_W as f32)))
        .with(StyleDeclaration::height(Length::Px(IMG_H as f32)))
        .with(StyleDeclaration::padding_top(Length::Px(40.0)))
        .with(StyleDeclaration::padding_bottom(Length::Px(40.0)))
        .with(StyleDeclaration::padding_left(Length::Px(30.0)))
        .with(StyleDeclaration::padding_right(Length::Px(30.0)))
        .with(bg_color(TRACK_BG));
    for decl in radius_all(LengthDefaultsToZero::Px(100.0)) {
        track_style = track_style.with(decl);
    }
    let track = Node::container(vec![bar]).with_style(track_style);

    render_node(track)
}

pub fn render_traffic_light() -> Vec<u8> {
    let mut lights = Vec::with_capacity(3);
    for color in [LIGHT_RED, LIGHT_YELLOW, LIGHT_GREEN] {
        let mut style = Style::default()
            .with(StyleDeclaration::width(Length::Px(120.0)))
            .with(StyleDeclaration::height(Length::Px(120.0)))
            .with(bg_color(color));
        for decl in radius_all(LengthDefaultsToZero::Percentage(50.0)) {
            style = style.with(decl);
        }
        lights.push(Node::container([]).with_style(style));
    }

    let mut housing_style = Style::default()
        .with(StyleDeclaration::display(Display::Flex))
        .with(StyleDeclaration::flex_direction(FlexDirection::Row))
        .with(StyleDeclaration::justify_content(JustifyContent::SpaceAround))
        .with(StyleDeclaration::align_items(AlignItems::Center))
        .with(StyleDeclaration::width(Length::Px(IMG_W as f32)))
        .with(StyleDeclaration::height(Length::Px(IMG_H as f32)))
        .with(bg_color(TRACK_BG));
    for decl in radius_all(LengthDefaultsToZero::Px(40.0)) {
        housing_style = housing_style.with(decl);
    }
    let housing = Node::container(lights).with_style(housing_style);

    render_node(housing)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_progress_bar_png() {
        let bytes = render_progress_bar();
        assert!(bytes.starts_with(&[0x89, b'P', b'N', b'G']));
    }

    #[test]
    fn renders_traffic_light_png() {
        let bytes = render_traffic_light();
        assert!(bytes.starts_with(&[0x89, b'P', b'N', b'G']));
    }
}
