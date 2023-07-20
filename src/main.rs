mod app;
mod crawler;

use std::{io, time::Duration};
use std::error::Error;
use std::io::Stdout;
use std::time::Instant;
use ratatui::{backend::CrosstermBackend, widgets::{Block, Borders}, Terminal, Frame};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::Backend;
use ratatui::prelude::*;
use ratatui::widgets::{List, ListItem, Tabs};
use anyhow::{Result};
use ratatui::symbols::DOT;
use crate::crawler::V2exTopic;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let res = run_app(&mut terminal, Duration::from_millis(250));

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}

#[derive(Clone, Debug, Default)]
pub struct AppState {
    current_page: usize,
    data: Option<Vec<V2exTopic>>,
    loading_state: usize,
}

impl AppState {
    pub fn new() -> Self {
        AppState { current_page: 1, loading_state: 1, data: None }
    }

    pub fn set_data(&mut self, data: Vec<V2exTopic>) {
        let _ = std::mem::replace(&mut self.data, Some(data));
    }
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<Stdout>>, tick_rate: Duration) -> Result<()> {
    let mut app_state = AppState::new();
    let mut last_tick = Instant::now();
    let mut should_quit = false;
    let current_page = 1;
    let (sender, receiver) = std::sync::mpsc::channel();

    tokio::task::spawn(async move {
        let Ok(page_html) = crawler::get_v2ex_page(current_page).await else {
            eprintln!("request {} v2ex page error", current_page);
            return;
        };

        let Ok(topic) = crawler::parse_v2ex_page(page_html) else {
            eprintln!("parse v2ex page {} error", current_page);
            return;
        };

        let _ = sender.send(topic);
    });

    loop {
        data(&mut app_state, &receiver);
        terminal.draw(|f| ui(f, &mut app_state))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if event::poll(timeout)? {
            if let event::Event::Key(event::KeyEvent { code, modifiers, kind, .. }) = event::read()? {
                if kind == event::KeyEventKind::Press {
                    match code {
                        event::KeyCode::Char('c') if modifiers == event::KeyModifiers::CONTROL => {
                            println!("Exiting...");
                            should_quit = true;
                            break;
                        }
                        // numbers
                        event::KeyCode::Char(c) if c.is_ascii_digit() => {
                            // open in browser
                            let index = c.to_digit(10).unwrap() as usize + 1;
                            let url = app_state.data.as_ref().unwrap()[index].get_topic_url();
                            open::that(url).unwrap();
                        }
                        _ => {}
                    }
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            // app.on_tick();
            last_tick = Instant::now();
        }
        if should_quit {
            break;
        }
    }

    Ok(())
}

fn data(app_state: &mut AppState, channel: &std::sync::mpsc::Receiver<Vec<V2exTopic>>) {
    let _ = channel.try_recv().map(|v| app_state.set_data(v));
}


fn ui<B: Backend>(f: &mut Frame<B>, app_state: &mut AppState) {
    let has_data = app_state.data.is_some();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            if has_data {
                vec![
                    Constraint::Percentage(15),
                    Constraint::Percentage(85),
                ]
            } else {
                vec![
                    Constraint::Percentage(100),
                ]
            }
        )
        .split(f.size());

    let Some(topics) = &app_state.data else {
        // 1 to 5 point count, each refresh increase number and reach 5 next to 1
        let loading_progress_text = format!("Loading{}", ".".repeat(app_state.loading_state));
        let loading_progress = ratatui::widgets::Paragraph::new(loading_progress_text)
            .style(Style::default().fg(Color::White).bg(Color::Black));
        app_state.loading_state = (app_state.loading_state + 1) % 5;
        f.render_widget(loading_progress, chunks[0]);
        return;
    };

    // todo tabs
    let titles = ["Tab1", "Tab2", "Tab3", "Tab4"].iter().cloned().map(Line::from).collect();
    let tabs = Tabs::new(titles)
        .block(Block::default().title("Tabs").borders(Borders::ALL))
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().fg(Color::Yellow))
        .divider(DOT);

    // fixme topic number should not greater than 10, or use other shortcut to open topic
    let list_items = topics.iter().enumerate().map(|(idx, t)| {
        ListItem::new(format!("{} \\ {}", idx + 1, t.list_item_format()))
    }).collect::<Vec<ListItem>>();
    let list = List::new(list_items)
        .block(Block::default().title(format!("Num {} Page", app_state.current_page)).borders(Borders::ALL))
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
        .highlight_symbol(">>");

    f.render_widget(tabs, chunks[0]);
    f.render_widget(list, chunks[1]);
}