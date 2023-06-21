use std::fs::File;
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use tokio::sync::Mutex;
use tracing::debug;

use memmap::{Mmap, MmapOptions};

use crate::{
    action::{Action, ActionHandler},
    components::{home::Home, Component},
    event::EventHandler,
    tui::Tui,
};

pub struct App {
    pub events: EventHandler,
    pub actions: ActionHandler,
    pub home: Arc<Mutex<Home>>,
    pub tui: Arc<Mutex<Tui>>,
}

impl App {
    pub fn new(tick_rate: u64, filename: String) -> Self {
        let tui = Arc::new(Mutex::new(
            Tui::new().context(anyhow!("Unable to create TUI")).unwrap(),
        ));
        let events = EventHandler::new(tick_rate);
        let actions = ActionHandler::new();
        let file = File::open(filename).unwrap();
        let mmap = unsafe { MmapOptions::new().map(&file).unwrap() };
        let home = Arc::new(Mutex::new(Home::new(mmap)));

        Self {
            tui,
            events,
            actions,
            home,
        }
    }

    pub async fn init(&mut self) -> Result<()> {
        self.home.lock().await.init()
    }

    pub async fn enter(&mut self) -> Result<()> {
        self.tui.lock().await.enter()?;
        Ok(())
    }

    pub async fn exit(&mut self) -> Result<()> {
        self.tui.lock().await.exit()?;
        Ok(())
    }

    pub async fn run(&mut self) -> Result<()> {
        let home = Arc::clone(&self.home);
        let tui = Arc::clone(&self.tui);
        tokio::spawn(async move {
            loop {
                let mut h = home.lock().await;
                let mut t = tui.lock().await;
                t.terminal
                    .draw(|f| {
                        h.render(f, f.size());
                    })
                    .unwrap();
            }
        });

        loop {
            let event = self.events.next().await;
            self.home
                .lock()
                .await
                .handle_events(event, &mut self.actions)
                .await?;
            let mut action = Some(self.actions.recv().await);
            while action.is_some() {
                action = self.home.lock().await.dispatch(action.unwrap());
            }
            if !(self.home.lock().await.is_running) {
                break;
            }
        }
        Ok(())
    }
}
