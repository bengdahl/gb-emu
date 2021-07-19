use iced::{Background, Color, Length};

#[cfg(not(target_arch = "wasm32"))]
use {
    iced_graphics::{Defaults, Primitive, Renderer},
    iced_native::{layout::Node, mouse, Layout, Point, Rectangle},
};

pub struct GameboyScreen {
    width: Length,
    height: Length,
}

impl GameboyScreen {
    pub fn new() -> Self {
        GameboyScreen {
            width: Length::Units(160),
            height: Length::Units(144),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl<M, B: iced_graphics::Backend> iced_native::widget::Widget<M, Renderer<B>> for GameboyScreen {
    fn width(&self) -> Length {
        self.width
    }

    fn height(&self) -> Length {
        self.height
    }

    fn layout(&self, _renderer: &Renderer<B>, _limits: &iced_native::layout::Limits) -> Node {
        Node::new(iced_native::Size::new(160.0, 144.0))
    }

    fn hash_layout(&self, state: &mut iced_native::Hasher) {
        use std::hash::Hash;
        self.width.hash(state);
        self.height.hash(state);
    }

    fn draw(
        &self,
        _renderer: &mut Renderer<B>,
        _defaults: &Defaults,
        layout: Layout<'_>,
        _cursor_position: Point,
        _viewport: &Rectangle,
    ) -> (Primitive, mouse::Interaction) {
        (
            Primitive::Quad {
                bounds: layout.bounds(),
                background: Background::Color(Color::BLACK),
                border_radius: 0.0,
                border_width: 0.0,
                border_color: Color::TRANSPARENT,
            },
            mouse::Interaction::Idle,
        )
    }
}

impl<'a, M, B: iced_graphics::Backend> Into<iced_native::Element<'a, M, Renderer<B>>>
    for GameboyScreen
{
    fn into(self) -> iced_native::Element<'a, M, Renderer<B>> {
        iced_native::Element::new(self)
    }
}
