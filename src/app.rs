use std::sync::mpsc::channel;
use std::sync::Arc;
use std::{fs::File, sync::mpsc};

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
    pub fn new(tick_rate: u64, filename: String, mmap: Mmap) -> Self {
        let arc_mmap = Arc::new(mmap);
        let tui = Arc::new(Mutex::new(
            Tui::new().context(anyhow!("Unable to create TUI")).unwrap(),
        ));
        let events = EventHandler::new(tick_rate);
        let actions = ActionHandler::new();

        let home = Arc::new(Mutex::new(Home::new(filename, arc_mmap)));

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
        let (tx, rx) = channel::<()>();
        // let (slow_thread_sender, slow_thread_receiver) = mpsc::unbounded_channel();
        tokio::spawn(async move {
            let mut sent = false;
            loop {
                let mut h = home.lock().await;
                let mut t = tui.lock().await;
                t.terminal
                    .draw(|f| {
                        h.render(f, f.area());
                    })
                    .unwrap();
                if !sent {
                    sent = true;
                    tx.send(()).unwrap();
                }
            }
        });

        rx.recv()?; // Wait for the first draw before we grab the home lock.
                    // It might be nice to allow actions to be processed without interrupting
                    // the main operations...
        loop {
            {
                // Block while holding home lock so we stop the draw thread.
                let mut home = self.home.lock().await;
                let event = self.events.next().await;
                home.handle_events(event, &mut self.actions).await?;
                let mut action = Some(self.actions.recv().await);
                while action.is_some() {
                    action = home.dispatch(action.unwrap());
                }
            }
            if !(self.home.lock().await.is_running) {
                break;
            }
        }
        Ok(())
    }
}
