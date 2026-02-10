use color_eyre::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, ListState},
    Terminal, TerminalOptions, Viewport,
    style::Styled, // Add this import
};
use std::io::{self, stdout};

struct BranchInfo {
    name: String,
    tracking_info: String,
    last_commit_date: String,
    last_commit_timestamp: i64,
    has_upstream: bool,
    is_current: bool,
}

struct App {
    branches: Vec<BranchInfo>,
    state: ListState,
    should_quit: bool,
    last_checked_out_branch: Option<String>,
}

impl App {
    fn new(branches: Vec<BranchInfo>) -> Self {
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

    let branches = get_branch_info()?;
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

    let initial_selection = app
        .branches
        .iter()
        .position(|b| b.is_current)
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

fn get_branch_info() -> io::Result<Vec<BranchInfo>> {
    const DELIMITER: &str = "|";
    let format = [
        "%(HEAD)",
        "%(refname:short)",
        "%(upstream:track,nobracket)",
        "%(committerdate:relative)",
        "%(committerdate:unix)",
        "%(upstream:short)",
    ]
    .join(DELIMITER);

    let output = std::process::Command::new("git")
        .args(["for-each-ref", &format!("--format={}", format), "refs/heads/"])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(io::Error::new(io::ErrorKind::Other, stderr.to_string()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut branches: Vec<BranchInfo> = stdout
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split(DELIMITER).collect();
            if parts.len() == 6 {
                let is_current = !parts[0].trim().is_empty();
                let timestamp = parts[4].parse::<i64>().unwrap_or(0);
                let has_upstream = !parts[5].trim().is_empty();
                Some(BranchInfo {
                    name: parts[1].to_string(),
                    tracking_info: parts[2].to_string(),
                    last_commit_date: parts[3].to_string(),
                    last_commit_timestamp: timestamp,
                    has_upstream,
                    is_current,
                })
            } else {
                None
            }
        })
        .collect();

    // Sort by last commit timestamp, newest first
    branches.sort_by(|a, b| b.last_commit_timestamp.cmp(&a.last_commit_timestamp));

    Ok(branches)
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
                    app.last_checked_out_branch = Some(app.branches[selected].name.clone());
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

        .map(|b| {



            let default_text_style = if !b.has_upstream || b.tracking_info.contains("gone") {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default()
            };

            let prefix_style = if b.is_current {
                Style::default().fg(Color::Green)
            } else {
                default_text_style // If not current, use default_text_style for prefix as well
            };
            let name_style = default_text_style.add_modifier(Modifier::BOLD);
            let date_style = default_text_style.fg(Color::Yellow);
            let tracking_style = default_text_style.fg(Color::Cyan);

            let line = Line::from(vec![
                Span::styled(if b.is_current { "* " } else { "  " }, prefix_style),
                Span::styled(&b.name, name_style),
                Span::raw(" (").set_style(default_text_style),
                Span::styled(&b.last_commit_date, date_style),
                Span::raw(") ").set_style(default_text_style),
                Span::styled(&b.tracking_info, tracking_style),
            ]);
            ListItem::new(line)
        })
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

    