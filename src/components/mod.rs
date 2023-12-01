use std::io::Stdout;

use anyhow::Result;
use crossterm::event::{KeyEvent, MouseEvent};
use ratatui::{backend::CrosstermBackend, layout::Rect, Frame as TuiFrame};

pub type Frame<'a> = TuiFrame<'a, CrosstermBackend<Stdout>>;

use crate::{
    action::{Action, ActionHandler},
    event::Event,
};

pub(crate) mod filter;
pub mod filter_screen;
pub mod go_screen;
pub mod home;
pub mod logger;
pub mod text_entry;

pub trait Component {
    // I wanted to add an argument to this but it will be a more involved refactor then i want to take at this moment.
    // At this time it's not clear why init is a property of the Component trait if there's never a call through the trait to it.
    fn init(&mut self) -> Result<()> {
        Ok(())
    }
    async fn handle_events(&self, event: Option<Event>, handler: &mut ActionHandler) -> Result<()> {
        match event {
            Some(Event::Quit) => handler.send(Action::Quit).await,
            Some(Event::Tick) => handler.send(Action::Tick).await,
            Some(Event::Key(key_event)) => self.handle_key_events(key_event, handler).await,
            Some(Event::Mouse(mouse_event)) => self.handle_mouse_events(mouse_event, handler).await,
            Some(Event::Resize(x, y)) => handler.send(Action::Resize(x, y)).await,
            Some(_) => handler.send(Action::Noop).await,
            None => handler.send(Action::Noop).await,
        }
    }
    async fn handle_key_events(&self, key: KeyEvent, handler: &mut ActionHandler) -> Result<()> {
        let action = self.on_key_event(key);
        handler.send(action).await
    }
    fn on_key_event(&self, key: KeyEvent) -> Action {
        Action::Noop
    }
    async fn handle_mouse_events(
        &self,
        mouse: MouseEvent,
        handler: &mut ActionHandler,
    ) -> Result<()> {
        let action = self.on_mouse_event(mouse);
        handler.send(action).await
    }
    fn on_mouse_event(&self, mouse: MouseEvent) -> Action {
        Action::Noop
    }
    fn dispatch(&mut self, action: Action) -> Option<Action> {
        None
    }
    fn render(&mut self, f: &mut Frame<'_>, rect: Rect);
}
