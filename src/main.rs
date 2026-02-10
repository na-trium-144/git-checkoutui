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
    last_checked_out_branch: Option<String>,
}

impl App {
    fn new(branches: Vec<String>) -> Self {
        Self {
            branches,
            state: ListState::default(),
            should_quit: false,
            last_checked_out_branch: None,
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

    let initial_selection = get_current_branch()
        .ok()
        .flatten()
        .and_then(|current| app.branches.iter().position(|b| b == &current))
        .or_else(|| if app.branches.is_empty() { None } else { Some(0) });

    if let Some(selected_index) = initial_selection {
        app.state.select(Some(selected_index));
    }

    run_app(&mut terminal, &mut app)?;

    // Restore terminal
    disable_raw_mode()?;

    if let Some(branch_name) = app.last_checked_out_branch {
        // run git checkout <branch_name>
        // and pipe the output to the parent terminal
        let mut command = std::process::Command::new("git");
        command.arg("checkout").arg(branch_name);
        command.stdout(std::process::Stdio::inherit());
        command.stderr(std::process::Stdio::inherit());
        let _ = command.status()?; // We can ignore the result, git will print errors.
    }

    Ok(())
}

fn get_current_branch() -> io::Result<Option<String>> {
    let output = std::process::Command::new("git")
        .args(["branch", "--show-current"])
        .output()?;
    if output.status.success() {
        let branch_name = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if branch_name.is_empty() {
            Ok(None) // Detached HEAD or other state
        } else {
            Ok(Some(branch_name))
        }
    } else {
        Ok(None)
    }
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
                if let Some(selected) = app.state.selected() {
                    app.last_checked_out_branch = Some(app.branches[selected].clone());
                }
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

    