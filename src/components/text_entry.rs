use anyhow::Result;
use crossterm::{
    cursor::MoveToPreviousLine,
    event::{KeyCode, KeyEvent},
};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets,
    widgets::{Block, Borders, List, ListItem, ListState, Widget},
};
use tracing::info;

use crate::action::Action;
use crate::action::LineFilter;

use tui_textarea::{CursorMove, TextArea};

use super::{Component, Frame};

#[derive(Default)]
pub struct TextEntry<'a> {
    // state: TuiWidgetState,
    pub textarea: TextArea<'a>,
}

impl TextEntry<'_> {
    pub fn contents(&self) -> &String {
        &self.textarea.lines()[0]
    }

    pub fn clear(&mut self) {
        self.textarea.move_cursor(CursorMove::End);
        self.textarea.delete_line_by_head();
    }

    pub fn pop(&mut self) -> String {
        let returning = self.contents().clone();
        self.clear();
        returning
    }
}

impl<'a> Component for TextEntry<'a> {
    fn init(&mut self) -> Result<()> {
        Ok(())
    }

    fn on_key_event(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Enter => Action::ConfirmTextEntry,
            KeyCode::Esc => Action::CloseTextEntry,
            keycode => Action::TextEntry(key),
            // _ => { self.textarea.input(key); Action::Tick },
            // _ => { self.textarea.withinput(key); Action::Tick },
            // TODO Send Key Action
        }
    }

    fn dispatch(&mut self, action: Action) -> Option<Action> {
        match action {
            Action::TextEntry(key) => {
                self.textarea.input(key);
                // info!("Received: {:?}", action);
            }
            _ => {}
        }
        None
    }

    fn render(&mut self, f: &mut Frame<'_>, rect: Rect) {
        // self.textarea.widget;
        // let tw = ;
        // f.render_widget::<dyn Widget>(tw, rect);
        // let w = ;
        /*let chunks = Layout::default()
          .direction(Direction::Vertical)
          .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
          .split(rect);
        self.filter_screen.render(f, chunks[1]);
        chunks[0]
        */
        /*
        let a: Box<dyn Any> = Box::new(w);
        let _: &tui_textarea::widget::Renderer = match a.downcast_ref::<Renderer>() {
            Some(b) => b,
            None => panic!("&a isn't a B!")
        };
        */
        f.render_widget(self.textarea.widget(), rect);
        // f.render_widget(self.textarea.widget(), rect);
        // f.render_stateful_widget(l, rect, &mut self.state);
    }
}
