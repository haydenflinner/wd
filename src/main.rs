use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use tracing::error;
use wd::{app::App, logging::initialize_logging, tui::Tui, utils::initialize_panic_handler};

/// Ratatui Template TUI
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// The tick rate to use
    #[arg(short, long, default_value_t = 5000)]
    tick_rate: u64,

    #[arg()]
    filename: String,
}

async fn tui_main(tick_rate: u64, filename: String) -> Result<()> {
    let mut app = App::new(tick_rate, filename);
    app.enter().await?;
    app.init().await?;
    app.run().await?;
    app.exit().await?;
    Ok(())
}

/*
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::widgets::{Block, Borders};
use ratatui::Terminal;
use std::io;
use tui_textarea::{Input, Key, TextArea};

fn main() -> io::Result<()> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    enable_raw_mode()?;
    crossterm::execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut term = Terminal::new(backend)?;

    let mut textarea = TextArea::default();
    textarea.set_block(
        Block::default()
            .borders(Borders::ALL)
            .title("Crossterm Minimal Example"),
    );

    loop {
        term.draw(|f| {
            f.render_widget(textarea.widget(), f.size());
        })?;
        match crossterm::event::read()?.into() {
            Input { key: Key::Esc, .. } => break,
            input => {
                textarea.input(input);
            }
        }
    }

    disable_raw_mode()?;
    crossterm::execute!(
        term.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    term.show_cursor()?;

    println!("Lines: {:?}", textarea.lines());
    Ok(())
}
*/

fn main() -> Result<()> {
    initialize_logging()?;

    initialize_panic_handler();

    let args = Args::parse();

    match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(async { tui_main(args.tick_rate, args.filename).await })
    {
        Ok(_) => std::process::exit(0),
        Err(e) => {
            match Tui::new() {
                Ok(tui) => {
                    if let Err(r) = tui.exit() {
                        error!("Unable to exit Tui: {r:?}");
                    }
                }
                Err(r) => error!("Unable to exit Tui: {r:?}"),
            }
            let s = "Error".red().bold();
            eprintln!("{s}: {e}");
            std::process::exit(1)
        }
    }
}
