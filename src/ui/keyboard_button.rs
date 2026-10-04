//! Iced 0.14's button handles pointer events only. This wrapper gives the same
//! control a native focus target, Enter/Space activation and a visible focus ring.
use iced::advanced::Renderer as _;
use iced::advanced::{
    Clipboard, Layout, Shell, Widget, layout, renderer,
    widget::{self, Tree, operation::Focusable, tree},
};
use iced::{Element, Event, Length, Rectangle, Size, Theme, keyboard, mouse};

pub fn keyboard_button<'a, Message: Clone + 'a>(
    id: impl Into<widget::Id>,
    label: String,
    message: Message,
) -> Element<'a, Message> {
    let inner = iced::widget::button(iced::widget::text(label))
        .on_press(message.clone())
        .style(iced::widget::button::secondary);
    Element::new(KeyboardButton {
        id: id.into(),
        inner: inner.into(),
        message,
    })
}

struct KeyboardButton<'a, Message> {
    id: widget::Id,
    inner: Element<'a, Message>,
    message: Message,
}

#[derive(Default)]
struct Focus {
    focused: bool,
}
impl Focusable for Focus {
    fn is_focused(&self) -> bool {
        self.focused
    }
    fn focus(&mut self) {
        self.focused = true;
    }
    fn unfocus(&mut self) {
        self.focused = false;
    }
}

impl<Message: Clone> Widget<Message, Theme, iced::Renderer> for KeyboardButton<'_, Message> {
    fn size(&self) -> Size<Length> {
        self.inner.as_widget().size()
    }
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Focus>()
    }
    fn state(&self) -> tree::State {
        tree::State::new(Focus::default())
    }
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.inner)]
    }
    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.inner));
    }
    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.inner
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }
    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        operation.focusable(
            Some(&self.id),
            layout.bounds(),
            tree.state.downcast_mut::<Focus>(),
        );
        self.inner
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
    }
    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let focus = tree.state.downcast_mut::<Focus>();
        if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event {
            focus.focused = cursor.is_over(layout.bounds());
        }
        if focus.focused
            && matches!(event, Event::Keyboard(keyboard::Event::KeyPressed { key: keyboard::Key::Named(keyboard::key::Named::Enter | keyboard::key::Named::Space), modifiers, .. }) if modifiers.is_empty())
        {
            shell.publish(self.message.clone());
            shell.capture_event();
            return;
        }
        self.inner.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }
    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.inner.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
        if tree.state.downcast_ref::<Focus>().focused {
            renderer.fill_quad(
                renderer::Quad {
                    bounds: layout.bounds(),
                    border: iced::Border {
                        color: theme.extended_palette().primary.strong.color,
                        width: 2.0,
                        radius: 4.0.into(),
                    },
                    ..renderer::Quad::default()
                },
                iced::Color::TRANSPARENT,
            );
        }
    }
    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.inner.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }
}
