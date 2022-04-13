use chrono::Utc;
use crossterm::{
    event::{self, KeyCode, KeyEvent, KeyModifiers},
    execute, terminal,
};
use eyre::Result;
use std::{
    io::{stdout, Write},
    ops::Sub,
    time::Duration,
};

use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Corner, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use unicode_width::UnicodeWidthStr;

use atuin_client::{
    database::Database,
    history::History,
    settings::{SearchMode, Settings},
};

const VERSION: &str = env!("CARGO_PKG_VERSION");

struct State {
    input: String,

    results: Vec<History>,

    results_state: ListState,
}

impl State {
    #[allow(clippy::cast_sign_loss)]
    fn durations(&self) -> Vec<(String, String)> {
        self.results
            .iter()
            .map(|h| {
                let duration =
                    Duration::from_millis(std::cmp::max(h.duration, 0) as u64 / 1_000_000);
                let duration = humantime::format_duration(duration).to_string();
                let duration: Vec<&str> = duration.split(' ').collect();

                let ago = chrono::Utc::now().sub(h.timestamp);

                // Account for the chance that h.timestamp is "in the future"
                // This would mean that "ago" is negative, and the unwrap here
                // would fail.
                // If the timestamp would otherwise be in the future, display
                // the time ago as 0.
                let ago = humantime::format_duration(
                    ago.to_std().unwrap_or_else(|_| Duration::new(0, 0)),
                )
                .to_string();
                let ago: Vec<&str> = ago.split(' ').collect();

                (
                    duration[0]
                        .to_string()
                        .replace("days", "d")
                        .replace("day", "d")
                        .replace("weeks", "w")
                        .replace("week", "w")
                        .replace("months", "mo")
                        .replace("month", "mo")
                        .replace("years", "y")
                        .replace("year", "y"),
                    ago[0]
                        .to_string()
                        .replace("days", "d")
                        .replace("day", "d")
                        .replace("weeks", "w")
                        .replace("week", "w")
                        .replace("months", "mo")
                        .replace("month", "mo")
                        .replace("years", "y")
                        .replace("year", "y")
                        + " ago",
                )
            })
            .collect()
    }

    fn render_results<T: tui::backend::Backend>(
        &mut self,
        f: &mut tui::Frame<T>,
        r: tui::layout::Rect,
        b: tui::widgets::Block,
    ) {
        let durations = self.durations();
        let max_length = durations.iter().fold(0, |largest, i| {
            std::cmp::max(largest, i.0.len() + i.1.len())
        });

        let results: Vec<ListItem> = self
            .results
            .iter()
            .enumerate()
            .map(|(i, m)| {
                let command = m.command.to_string().replace('\n', " ").replace('\t', " ");

                let mut command = Span::raw(command);

                let (duration, mut ago) = durations[i].clone();

                while (duration.len() + ago.len()) < max_length {
                    ago = format!(" {}", ago);
                }

                let selected_index = match self.results_state.selected() {
                    None => Span::raw("   "),
                    Some(selected) => match i.checked_sub(selected) {
                        None => Span::raw("   "),
                        Some(diff) => {
                            if 0 < diff && diff < 10 {
                                Span::raw(format!(" {} ", diff))
                            } else {
                                Span::raw("   ")
                            }
                        }
                    },
                };

                let duration = Span::styled(
                    duration,
                    Style::default().fg(if m.exit == 0 || m.duration == -1 {
                        Color::Green
                    } else {
                        Color::Red
                    }),
                );

                let ago = Span::styled(ago, Style::default().fg(Color::Blue));

                if let Some(selected) = self.results_state.selected() {
                    if selected == i {
                        command.style =
                            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD);
                    }
                }

                let spans = Spans::from(vec![
                    selected_index,
                    duration,
                    Span::raw(" "),
                    ago,
                    Span::raw(" "),
                    command,
                ]);

                ListItem::new(spans)
            })
            .collect();

        let results = List::new(results)
            .block(b)
            .start_corner(Corner::BottomLeft)
            .highlight_symbol(">> ");

        f.render_stateful_widget(results, r, &mut self.results_state);
    }
}

async fn query_results(
    app: &mut State,
    search_mode: SearchMode,
    db: &mut (impl Database + Send + Sync),
) -> Result<()> {
    let results = match app.input.as_str() {
        "" => db.list(Some(200), true).await?,
        i => db.search(Some(200), search_mode, i).await?,
    };

    app.results = results;

    if app.results.is_empty() {
        app.results_state.select(None);
    } else {
        app.results_state.select(Some(0));
    }

    Ok(())
}

async fn key_handler(
    input: KeyEvent,
    search_mode: SearchMode,
    db: &mut (impl Database + Send + Sync),
    app: &mut State,
) -> Option<String> {
    match input.code {
        KeyCode::Esc => return Some(String::from("")),
        KeyCode::Char('c' | 'd' | 'g') if input.modifiers.contains(KeyModifiers::CONTROL) => {
            return Some(String::from(""))
        }
        KeyCode::Enter => {
            let i = app.results_state.selected().unwrap_or(0);

            return Some(
                app.results
                    .get(i)
                    .map_or(app.input.clone(), |h| h.command.clone()),
            );
        }
        KeyCode::Down => app.down(),
        KeyCode::Char('n') if input.modifiers.contains(KeyModifiers::CONTROL) => app.down(),
        KeyCode::Up => app.up(),
        KeyCode::Char('p') if input.modifiers.contains(KeyModifiers::CONTROL) => app.up(),
        KeyCode::Char(c @ '1'..='9') if input.modifiers.contains(KeyModifiers::ALT) => {
            let c = c.to_digit(10)? as usize;
            let i = app.results_state.selected()? + c;

            return Some(
                app.results
                    .get(i)
                    .map_or(app.input.clone(), |h| h.command.clone()),
            );
        }
        KeyCode::Backspace => {
            app.input.pop();
            query_results(app, search_mode, db).await.unwrap();
        }
        // \u{7f} is escape sequence for backspace
        KeyCode::Char('\u{7f}') if input.modifiers.contains(KeyModifiers::ALT) => {
            let words: Vec<&str> = app.input.split(' ').collect();
            if words.is_empty() {
                return None;
            }
            if words.len() == 1 {
                app.input = String::from("");
            } else {
                app.input = words[0..(words.len() - 1)].join(" ");
            }
            query_results(app, search_mode, db).await.unwrap();
        }
        KeyCode::Char('u') if input.modifiers.contains(KeyModifiers::CONTROL) => {
            app.input = String::from("");
            query_results(app, search_mode, db).await.unwrap();
        }
        KeyCode::Char(c) => {
            app.input.push(c);
            query_results(app, search_mode, db).await.unwrap();
        }
        _ => {}
    };

    None
}

impl State {
    fn down(&mut self) {
        let i = match self.results_state.selected() {
            Some(i) => i.saturating_sub(1),
            None => 0,
        };
        self.results_state.select(Some(i));
    }
    fn up(&mut self) {
        let i = match self.results_state.selected() {
            Some(i) => (i + 1).min(self.results.len() - 1),
            None => 0,
        };
        self.results_state.select(Some(i));
    }
}

#[allow(clippy::cast_possible_truncation)]
fn draw<T: Backend>(f: &mut Frame<'_, T>, history_count: i64, app: &mut State) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Length(2),
                Constraint::Min(1),
                Constraint::Length(3),
            ]
            .as_ref(),
        )
        .split(f.size());

    let top_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(chunks[0]);

    let top_left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)].as_ref())
        .split(top_chunks[0]);

    let top_right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)].as_ref())
        .split(top_chunks[1]);

    let title = Paragraph::new(Text::from(Span::styled(
        format!("Atuin v{}", VERSION),
        Style::default().add_modifier(Modifier::BOLD),
    )));

    let help = vec![
        Span::raw("Press "),
        Span::styled("Esc", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" to exit."),
    ];

    let help = Text::from(Spans::from(help));
    let help = Paragraph::new(help);

    let input = Paragraph::new(app.input.clone())
        .block(Block::default().borders(Borders::ALL).title("Query"));

    let stats = Paragraph::new(Text::from(Span::raw(format!(
        "history count: {}",
        history_count,
    ))))
    .alignment(Alignment::Right);

    f.render_widget(title, top_left_chunks[0]);
    f.render_widget(help, top_left_chunks[1]);
    f.render_widget(stats, top_right_chunks[0]);

    app.render_results(
        f,
        chunks[1],
        Block::default().borders(Borders::ALL).title("History"),
    );
    f.render_widget(input, chunks[2]);

    f.set_cursor(
        // Put cursor past the end of the input text
        chunks[2].x + app.input.width() as u16 + 1,
        // Move one line down, from the border to the input line
        chunks[2].y + 1,
    );
}

#[allow(clippy::cast_possible_truncation)]
fn draw_compact<T: Backend>(f: &mut Frame<'_, T>, history_count: i64, app: &mut State) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(0)
        .horizontal_margin(1)
        .constraints(
            [
                Constraint::Length(1),
                Constraint::Min(1),
                Constraint::Length(1),
            ]
            .as_ref(),
        )
        .split(f.size());

    let header_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Ratio(1, 3),
                Constraint::Ratio(1, 3),
                Constraint::Ratio(1, 3),
            ]
            .as_ref(),
        )
        .split(chunks[0]);

    let title = Paragraph::new(Text::from(Span::styled(
        format!("Atuin v{}", VERSION),
        Style::default().fg(Color::DarkGray),
    )));

    let help = Paragraph::new(Text::from(Spans::from(vec![
        Span::styled("Esc", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" to exit"),
    ])))
    .style(Style::default().fg(Color::DarkGray))
    .alignment(Alignment::Center);

    let stats = Paragraph::new(Text::from(Span::raw(format!(
        "history count: {}",
        history_count,
    ))))
    .style(Style::default().fg(Color::DarkGray))
    .alignment(Alignment::Right);

    let input = Paragraph::new(format!("] {}", app.input.clone())).block(Block::default());

    f.render_widget(title, header_chunks[0]);
    f.render_widget(help, header_chunks[1]);
    f.render_widget(stats, header_chunks[2]);

    app.render_results(f, chunks[1], Block::default());
    f.render_widget(input, chunks[2]);

    f.set_cursor(
        // Put cursor past the end of the input text
        chunks[2].x + app.input.width() as u16 + 2,
        // Move one line down, from the border to the input line
        chunks[2].y + 1,
    );
}

struct Stdout {
    stdout: std::fs::File,
}

impl Stdout {
    pub fn new() -> std::io::Result<Self> {
        terminal::enable_raw_mode()?;
        // let mut stdout = stdout();
        let mut stdout = std::fs::OpenOptions::new().read(true).write(true).open("/dev/tty")?;
        execute!(stdout, terminal::EnterAlternateScreen)?;
        // execute!(stdout, event::EnableMouseCapture)?;
        Ok(Self { stdout })
    }
}

impl Drop for Stdout {
    fn drop(&mut self) {
        execute!(
            self.stdout,
            terminal::LeaveAlternateScreen
        )
        .unwrap();
        terminal::disable_raw_mode().unwrap();
    }
}

impl Write for Stdout {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.stdout.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.stdout.flush()
    }
}

// this is a big blob of horrible! clean it up!
// for now, it works. But it'd be great if it were more easily readable, and
// modular. I'd like to add some more stats and stuff at some point
#[allow(clippy::cast_possible_truncation)]
async fn select_history(
    query: &[String],
    search_mode: SearchMode,
    style: atuin_client::settings::Style,
    db: &mut (impl Database + Send + Sync),
) -> Result<String> {
    let stdout = Stdout::new()?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = State {
        input: query.join(" "),
        results: Vec::new(),
        results_state: ListState::default(),
    };

    query_results(&mut app, search_mode, db).await?;

    loop {
        let history_count = db.history_count().await?;
        // Handle input
        if event::poll(Duration::from_millis(250))? {
            if let event::Event::Key(input) = event::read()? {
                if let Some(output) = key_handler(input, search_mode, db, &mut app).await {
                    return Ok(output);
                }
            }
        }

        let compact = match style {
            atuin_client::settings::Style::Auto => {
                terminal.size().map(|size| size.height < 14).unwrap_or(true)
            }
            atuin_client::settings::Style::Compact => true,
            atuin_client::settings::Style::Full => false,
        };
        if compact {
            terminal.draw(|f| draw_compact(f, history_count, &mut app))?;
        } else {
            terminal.draw(|f| draw(f, history_count, &mut app))?;
        }
    }
}

// This is supposed to more-or-less mirror the command line version, so ofc
// it is going to have a lot of args
#[allow(clippy::too_many_arguments)]
pub async fn run(
    settings: &Settings,
    cwd: Option<String>,
    exit: Option<i64>,
    interactive: bool,
    human: bool,
    exclude_exit: Option<i64>,
    exclude_cwd: Option<String>,
    before: Option<String>,
    after: Option<String>,
    cmd_only: bool,
    query: &[String],
    db: &mut (impl Database + Send + Sync),
) -> Result<()> {
    let dir = if let Some(cwd) = cwd {
        if cwd == "." {
            let current = std::env::current_dir()?;
            let current = current.as_os_str();
            let current = current.to_str().unwrap();

            Some(current.to_owned())
        } else {
            Some(cwd)
        }
    } else {
        None
    };

    if interactive {
        let item = select_history(query, settings.search_mode, settings.style, db).await?;
        eprintln!("{}", item);
    } else {
        let results = db
            .search(None, settings.search_mode, query.join(" ").as_str())
            .await?;

        // TODO: This filtering would be better done in the SQL query, I just
        // need a nice way of building queries.
        let results: Vec<History> = results
            .iter()
            .filter(|h| {
                if let Some(exit) = exit {
                    if h.exit != exit {
                        return false;
                    }
                }

                if let Some(exit) = exclude_exit {
                    if h.exit == exit {
                        return false;
                    }
                }

                if let Some(cwd) = &exclude_cwd {
                    if h.cwd.as_str() == cwd.as_str() {
                        return false;
                    }
                }

                if let Some(cwd) = &dir {
                    if h.cwd.as_str() != cwd.as_str() {
                        return false;
                    }
                }

                if let Some(before) = &before {
                    let before = chrono_english::parse_date_string(
                        before.as_str(),
                        Utc::now(),
                        chrono_english::Dialect::Uk,
                    );

                    if before.is_err() || h.timestamp.gt(&before.unwrap()) {
                        return false;
                    }
                }

                if let Some(after) = &after {
                    let after = chrono_english::parse_date_string(
                        after.as_str(),
                        Utc::now(),
                        chrono_english::Dialect::Uk,
                    );

                    if after.is_err() || h.timestamp.lt(&after.unwrap()) {
                        return false;
                    }
                }

                true
            })
            .map(std::borrow::ToOwned::to_owned)
            .collect();

        super::history::print_list(&results, human, cmd_only);
    }

    Ok(())
}
