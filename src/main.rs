use color_eyre::Result;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::error::Error;
use std::io;

use slurmboard::app::App;
use slurmboard::args::Args;
use slurmboard::event::{Event, EventHandler};
use slurmboard::handler::{handle_key_events, handle_mouse_events};
use slurmboard::tui::Tui;
use slurmboard::ui::UI;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Args = argh::from_env();
    if args.version {
        println!("slurmboard v{}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    let mut app = App::new(args)?;
    let mut ui = UI::new(&app);

    // Initialize the terminal user interface
    let backend = CrosstermBackend::new(io::stderr());
    let terminal = Terminal::new(backend)?;
    let events = EventHandler::new(50);
    let mut tui = Tui::new(terminal, events);
    tui.init()?;
    tui.draw(&mut ui)?;

    // Main loop
    while app.running {
        let redraw = match tui.events.next()? {
            Event::Tick => {
                if app.tick()? {
                    ui.update(&app);
                    true
                } else {
                    false
                }
            }
            Event::Key(key_event) => handle_key_events(key_event, &mut app, &mut ui)?,
            Event::Mouse(mouse_event) => handle_mouse_events(mouse_event, &mut ui)?,
            Event::Resize(_, _) => true,
        };

        // FIXME: More fine-grained checks
        if redraw {
            tui.draw(&mut ui)?;
        }
    }

    tui.exit()?;
    Ok(())
}
