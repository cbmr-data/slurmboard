use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};

use color_eyre::{config::HookBuilder, eyre, Result};
use ratatui::backend::Backend;
use ratatui::Terminal;
use std::error::Error;
use std::io;
use std::panic;

use crate::event::EventHandler;
use crate::ui::UI;

/// Representation of a terminal user interface.
///
/// It is responsible for setting up the terminal,
/// initializing the interface and handling the draw events.
#[derive(Debug)]
pub struct Tui<B: Backend>
where
    <B as Backend>::Error: 'static,
{
    /// Interface to the Terminal.
    terminal: Terminal<B>,
    /// Terminal event handler.
    pub events: EventHandler,
}

impl<B: Backend> Tui<B>
where
    <B as Backend>::Error: 'static,
{
    /// Constructs a new instance of [`Tui`].
    pub fn new(terminal: Terminal<B>, events: EventHandler) -> Self {
        Self { terminal, events }
    }

    /// Initializes the terminal interface.
    ///
    /// It enables the raw mode and sets terminal properties.
    pub fn init(&mut self) -> Result<(), Box<dyn Error>> {
        // Define a custom panic hook to reset the terminal properties.
        // This way, you won't have your terminal messed up if an unexpected error happens.
        let (panic_hook, eyre_hook) = HookBuilder::default().into_hooks();
        let panic_hook = panic_hook.into_panic_hook();
        panic::set_hook(Box::new(move |panic| {
            Self::reset().expect("failed to reset the terminal");
            panic_hook(panic);
        }));

        let eyre_hook = eyre_hook.into_eyre_hook();
        eyre::set_hook(Box::new(
            move |error: &(dyn std::error::Error + 'static)| {
                Self::reset().expect("failed to reset the terminal");
                eyre_hook(error)
            },
        ))?;

        terminal::enable_raw_mode()?;
        crossterm::execute!(io::stderr(), EnterAlternateScreen, EnableMouseCapture)?;

        self.terminal.hide_cursor()?;
        self.terminal.clear()?;
        Ok(())
    }

    /// [`Draw`] the terminal interface by [`rendering`] the widgets.
    ///
    /// [`Draw`]: ratatui::Terminal::draw
    pub fn draw(&mut self, ui: &mut UI) -> Result<(), Box<dyn Error>> {
        self.terminal
            .draw(|frame| ui.render(frame.area(), frame.buffer_mut()))?;

        Ok(())
    }

    /// Resets the terminal interface.
    ///
    /// This function is also used for the panic hook to revert
    /// the terminal properties if unexpected errors occur.
    fn reset() -> Result<()> {
        terminal::disable_raw_mode()?;
        crossterm::execute!(io::stderr(), LeaveAlternateScreen, DisableMouseCapture)?;
        Ok(())
    }

    /// Exits the terminal interface.
    ///
    /// It disables the raw mode and reverts back the terminal properties.
    pub fn exit(&mut self) -> Result<(), Box<dyn Error>> {
        Self::reset()?;
        self.terminal.show_cursor()?;
        Ok(())
    }
}
