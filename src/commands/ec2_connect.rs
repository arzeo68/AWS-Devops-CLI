use crate::commands::aws_utils::{list_ec2_instances};
use aws_sdk_ec2 as ec2;
use ratatui::crossterm::event;
use ratatui::crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::{Constraint, Layout, Alignment},
    widgets::{Block, List, ListItem, ListState, Paragraph},
    Frame,
    style::{Style, Modifier, Color},
};

use ratatui::text::{Span, Line};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Page {
    Instance = 0,
}

impl Page {
    fn next(self) -> Self {
        match self {
            Page::Instance => Page::Instance,
        }
    }
    fn prev(self) -> Self {
        match self {
            Page::Instance => Page::Instance,
        }
    }
    fn title(&self) -> &'static str {
        match self {
            // repurpose the first page to show EC2 instances
            Page::Instance => "Instances",
        }
    }
}

struct AppState {
    page: Page,
    // `clusters` will hold instance display names
    instance_names: Vec<String>,
    instance_ids: Vec<String>,
    idx_instance: usize,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            page: Page::Instance,
            instance_names: Vec::new(),
            instance_ids: Vec::new(),
            idx_instance: 0,
        }
    }
}

fn clamp_index(idx: usize, len: usize) -> usize {
    if len == 0 { 0 } else { idx.min(len - 1) }
}

async fn handle_events(state: &mut AppState) -> std::io::Result<bool> {
    match event::read()? {
        Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
            KeyCode::Char('q') => return Ok(true),
            KeyCode::Left => {
                state.page = state.page.prev();
            }
            KeyCode::Right => {
                state.page = state.page.next();
            }
            KeyCode::Up => {
                match state.page {
                    Page::Instance => if state.idx_instance > 0 { state.idx_instance -= 1 },
                }
            }
            KeyCode::Down => {
                match state.page {
                    Page::Instance => state.idx_instance = clamp_index(state.idx_instance + 1, state.instance_names.len()),
                }
            }
            KeyCode::Char('c') => {
                // connect to EC2 when on the Instances page (Cluster repurposed)
                if state.page == Page::Instance {
                    if state.instance_names.is_empty() {
                        return Ok(false);
                    }
                    let idx = state.idx_instance;
                    let target = &state.instance_ids[idx];
                    ratatui::restore();
                    connect_to_ec2_command(&target).await;
                    return Ok(true);
                }
            }

            KeyCode::Char('p') => {
                if state.page == Page::Instance {
                    if state.instance_names.is_empty() {
                        return Ok(false);
                    }
                    let idx = state.idx_instance;
                    let target = &state.instance_ids[idx];
                    ratatui::restore();
                    let host = crate::commands::port_forward::select_host(&"What host do you want to use?".to_string());
                    let remote_port = crate::commands::port_forward::select_port(&"What remote port do you want to use?".to_string());
                    let local_port = crate::commands::port_forward::select_port(&"What local port do you want to use?".to_string());
                    crate::commands::port_forward::connect_to_ecs_command(&target, &host, &local_port, &remote_port).await;
                    return Ok(true);
                }
            }


            KeyCode::Enter => {
                state.page = state.page.next();
            }
            _ => {}
        },
        _ => {}
    }
    Ok(false)
}

fn draw_list_block<'a>(title: &'a str, items: &'a [String], selected: usize) -> (List<'a>, ListState) {
    let list_items: Vec<ListItem> = items.iter().map(|i| ListItem::new(i.clone())).collect();
    let mut ls = ListState::default();
    if !items.is_empty() {
        ls.select(Some(clamp_index(selected, items.len())));
    } else {
        ls.select(None);
    }
    let list = List::new(list_items)
        .block(Block::bordered().title(title))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD).fg(Color::Yellow));
    (list, ls)
}

fn draw_ecs_connect(frame: &mut Frame, state: &AppState) {
    use Constraint::{Fill, Length, Min};

    let vertical = Layout::vertical([Min(0), Length(3)]);
    let [main_area, status_area] = vertical.areas(frame.area());
    let horizontal = Layout::horizontal([Fill(1); 2]);
    let [left_area, right_area] = horizontal.areas(main_area);


    // left: current page list
    let (list, mut list_state) = match state.page {
        Page::Instance => draw_list_block(Page::Instance.title(), &state.instance_names, state.idx_instance),
    };
    frame.render_stateful_widget(list, left_area, &mut list_state);

    // right: details / selection summary as Vec<Line>
    let mut details = vec![
        Line::from(Span::raw(format!("Page: {}", state.page.title()))),
        Line::from(""),
        Line::from(Span::raw(format!("Instance:  {}", state.instance_names.get(state.idx_instance).unwrap_or(&"None".to_string())))),
        Line::from(""),
        Line::from("Use ←/→ to change page, ↑/↓ to move selection, Enter to advance, q to quit."),
        Line::from("Use c to connect to the instance"),
        Line::from("Use p to port forward to the instance"),

    ];
    let para = Paragraph::new(details).block(Block::bordered().title("Details"));
    frame.render_widget(para, right_area);

    // Footer: four boxes, one per page, highlight the current one
    let footer_chunks = Layout::horizontal([
        Constraint::Percentage(25),
        Constraint::Percentage(25),
        Constraint::Percentage(25),
        Constraint::Percentage(25),
    ])
        .split(status_area);

    let sel_style = Style::default().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD);
    let normal_style = Style::default();

    let pages = [
        (Page::Instance, Page::Instance.title()),
    ];

    for (i, (page_enum, title)) in pages.iter().enumerate() {
        let is_sel = *page_enum == state.page;
        let text = Span::styled(title.to_string(), if is_sel { sel_style } else { normal_style });
        let mut block = Block::bordered();
        if is_sel {
            block = block.style(sel_style);
        }
        let p = Paragraph::new(text).alignment(Alignment::Center).block(block);
        frame.render_widget(p, footer_chunks[i]);
    }
}

pub async fn run_ec2_connect(terminal: &mut ratatui::DefaultTerminal) -> std::io::Result<()> {
    // initial state - load EC2 instances into the first page
    let mut state = AppState::default();

    let config = aws_config::load_from_env().await;
    let client = ec2::Client::new(&config);
    let instances = list_ec2_instances(&client).await;
    state.instance_names = instances.iter().map(|i| i.name.clone()).collect();
    state.instance_ids = instances.iter().map(|i| i.instance_id.clone()).collect();


    loop {
        // pass the state reference into the draw closure
        terminal.draw(|frame| draw_ecs_connect(frame, &state))?;
        if handle_events(&mut state).await? {
            break Ok(());
        }

        // No lazy loading for EC2 UI at the moment
    }
}

async fn connect_to_ec2_command(target: &str) {
    ctrlc::set_handler(move || {}).expect("Error setting Ctrl-C handler");

    let command = format!("aws ssm start-session --target {}", target);

    println!("{}", command);

    let output = std::process::Command::new("/bin/sh")
        .arg("-c")
        .arg(command)
        .spawn()
        .expect("failed to execute process");
    let _ = output.wait_with_output();
}


pub async fn ec2_connect() {
    let mut terminal = ratatui::init();
    run_ec2_connect(&mut terminal).await.expect("Can't connect to ec2");
    ratatui::restore();
}