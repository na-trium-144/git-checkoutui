use color_eyre::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, ListState},
    Terminal, TerminalOptions, Viewport,
};
use std::io::{self, stdout};

struct App {
    branches: Vec<String>,
    state: ListState,
    should_quit: bool,
}

impl App {
    fn new(branches: Vec<String>) -> Self {
        Self {
            branches,
            state: ListState::default(),
            should_quit: false,
        }
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.branches.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.branches.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn quit(&mut self) {
        self.should_quit = true;
    }

    fn checkout_selected(&self) -> io::Result<()> {
        if let Some(selected) = self.state.selected() {
            let branch_name = &self.branches[selected];
            checkout_branch(branch_name)?;
        }
        Ok(())
    }
}

fn main() -> Result<()> {
    color_eyre::install()?;

    let branches = get_git_branches()?;
    let height = if branches.is_empty() {
        3
    } else {
        branches.len().saturating_add(2).min(20) as u16 // 2 for borders, max 20
    };

    // Terminal initialization for inline rendering
    enable_raw_mode()?;
    let mut terminal = Terminal::with_options(
        CrosstermBackend::new(stdout()),
        TerminalOptions {
            viewport: Viewport::Inline(height),
        },
    )?;

    let mut app = App::new(branches);
    if !app.branches.is_empty() {
        app.state.select(Some(0));
    }

    run_app(&mut terminal, &mut app)?;

    // Restore terminal
    disable_raw_mode()?;
    Ok(())
}

fn get_git_branches() -> io::Result<Vec<String>> {
    let output = std::process::Command::new("git")
        .args(["branch", "--format=%(refname:short)"])
        .output()?;
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.lines().map(String::from).collect())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(io::Error::new(io::ErrorKind::Other, stderr.to_string()))
    }
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> io::Result<()> {
    while !app.should_quit {
        terminal.draw(|f| ui(f, app))?;
        handle_events(app)?;
    }
    Ok(())
}

fn handle_events(app: &mut App) -> io::Result<()> {
    if let Event::Key(key) = event::read()? {
        match key.code {
            KeyCode::Char('q') => app.quit(),
            KeyCode::Char('c') if key.modifiers == KeyModifiers::CONTROL => app.quit(),
            KeyCode::Down | KeyCode::Char('j') => app.next(),
            KeyCode::Up | KeyCode::Char('k')=> app.previous(),
            KeyCode::Enter => {
                app.checkout_selected()?;
                app.quit();
            }
            _ => {}
        }
    }
    Ok(())
}

fn ui(f: &mut Frame, app: &mut App) {
    if app.branches.is_empty() {
        let text = "No git branches found in this directory.";
        let block = Block::default()
            .title("Error")
            .borders(Borders::ALL);
        let paragraph = ratatui::widgets::Paragraph::new(text).block(block);
        f.render_widget(paragraph, f.area());
        return;
    }
    let items: Vec<ListItem> = app
        .branches
        .iter()
        .map(|i| ListItem::new(i.as_str()))
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Branches"))
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::REVERSED)
                .fg(Color::Green),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(list, f.area(), &mut app.state);
}

fn checkout_branch(branch_name: &str) -> io::Result<()> {
    let output = std::process::Command::new("git")
        .args(["checkout", branch_name])
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("Error checking out branch: {}", stderr); // Print to stderr for visibility after exit
        return Err(io::Error::new(io::ErrorKind::Other, stderr.to_string()));
    }
    println!("Switched to branch '{}'", branch_name);
    Ok(())
}