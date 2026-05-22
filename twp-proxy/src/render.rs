// Renders the hello-world shapes with the takumi layout engine.
//
// Approach A from the build plan: each "shape" is just a colored box. The
// goal of this step is to prove takumi is wired into the pipeline, not to
// produce pretty SVGs — that can come later once the protocol is real.

use std::borrow::Cow;
use std::sync::OnceLock;

use takumi::{
    GlobalContext,
    layout::{
        Viewport,
        node::Node,
        style::{Color, ColorInput, Length, Style, StyleDeclaration},
    },
    rendering::{ImageOutputFormat, RenderOptions, render, write_image},
};

pub const IMG_W: u32 = 200;
pub const IMG_H: u32 = 100;

fn context() -> &'static GlobalContext {
    static CTX: OnceLock<GlobalContext> = OnceLock::new();
    CTX.get_or_init(GlobalContext::default)
}

fn render_box(rgba: [u8; 4]) -> Vec<u8> {
    let color = Color::from(rgba);
    let style = Style::default()
        .with(StyleDeclaration::width(Length::Px(IMG_W as f32)))
        .with(StyleDeclaration::height(Length::Px(IMG_H as f32)))
        .with(StyleDeclaration::background_color(ColorInput::from(color)));

    let node = Node::container([]).with_style(style);

    let opts = RenderOptions::builder()
        .viewport(Viewport::new((IMG_W, IMG_H)))
        .node(node)
        .global(context())
        .build();

    let img = render(opts).expect("takumi render");

    let mut buf = Vec::with_capacity(8 * 1024);
    write_image(Cow::Owned(img), &mut buf, ImageOutputFormat::Png, None)
        .expect("takumi png encode");
    buf
}

pub fn render_triangle() -> Vec<u8> {
    render_box([0, 200, 64, 255])
}

pub fn render_circle() -> Vec<u8> {
    render_box([220, 32, 32, 255])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_triangle_png() {
        let bytes = render_triangle();
        assert!(bytes.starts_with(&[0x89, b'P', b'N', b'G']));
        eprintln!("triangle PNG: {} bytes", bytes.len());
    }

    #[test]
    fn renders_circle_png() {
        let bytes = render_circle();
        assert!(bytes.starts_with(&[0x89, b'P', b'N', b'G']));
        eprintln!("circle PNG: {} bytes", bytes.len());
    }
}
