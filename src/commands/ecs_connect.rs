use crate::commands::aws_utils::{ecs_execute_command, get_clusters, list_cluster_services, list_service_tasks, list_task_container};
use ratatui::crossterm::event;
use ratatui::crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::{Constraint, Layout, Alignment},
    widgets::{Block, List, ListItem, ListState, Paragraph},
    Frame,
    style::{Style, Modifier, Color},
};

use ratatui::text::{Span, Line};

pub struct AwsResource {
    pub(crate) arn: String,
    pub(crate) name: String,
}

pub struct ECSContainer {
    pub(crate) name: String,
    pub(crate) runtime_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Page {
    Cluster = 0,
    Services = 1,
    Tasks = 2,
    Container = 3,
}

impl Page {
    fn next(self) -> Self {
        match self {
            Page::Cluster => Page::Services,
            Page::Services => Page::Tasks,
            Page::Tasks => Page::Container,
            Page::Container => Page::Container,
        }
    }
    fn prev(self) -> Self {
        match self {
            Page::Cluster => Page::Cluster,
            Page::Services => Page::Cluster,
            Page::Tasks => Page::Services,
            Page::Container => Page::Tasks,
        }
    }
    fn title(&self) -> &'static str {
        match self {
            Page::Cluster => "Clusters",
            Page::Services => "Services",
            Page::Tasks => "Tasks",
            Page::Container => "Containers",
        }
    }
}

struct AppState {
    page: Page,
    clusters: Vec<String>,
    services: Vec<String>,
    tasks: Vec<String>,
    containers: Vec<String>,
    runtime_ids: Vec<String>,
    idx_cluster: usize,
    idx_service: usize,
    idx_task: usize,
    idx_container: usize,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            page: Page::Cluster,
            clusters: Vec::new(),
            services: Vec::new(),
            tasks: Vec::new(),
            containers: Vec::new(),
            runtime_ids: Vec::new(),
            idx_cluster: 0,
            idx_service: 0,
            idx_task: 0,
            idx_container: 0,
        }
    }
}

fn clamp_index(idx: usize, len: usize) -> usize {
    if len == 0 { 0 } else { idx.min(len - 1) }
}

fn reset_following(state: &mut AppState, page: Page) {
    match page {
        Page::Cluster => {
            state.services.clear();
            state.idx_service = 0;
            state.tasks.clear();
            state.idx_task = 0;
            state.containers.clear();
            state.idx_container = 0;
        }
        Page::Services => {
            state.tasks.clear();
            state.idx_task = 0;
            state.containers.clear();
            state.idx_container = 0;
        }
        Page::Tasks => {
            state.containers.clear();
            state.idx_container = 0;
        }
        Page::Container => {}
    }
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
                    Page::Cluster => if state.idx_cluster > 0 { state.idx_cluster -= 1 },
                    Page::Services => if state.idx_service > 0 { state.idx_service -= 1 },
                    Page::Tasks => if state.idx_task > 0 { state.idx_task -= 1 },
                    Page::Container => if state.idx_container > 0 { state.idx_container -= 1 },
                }
                reset_following(state, state.page);
            }
            KeyCode::Down => {
                match state.page {
                    Page::Cluster => state.idx_cluster = clamp_index(state.idx_cluster + 1, state.clusters.len()),
                    Page::Services => state.idx_service = clamp_index(state.idx_service + 1, state.services.len()),
                    Page::Tasks => state.idx_task = clamp_index(state.idx_task + 1, state.tasks.len()),
                    Page::Container => state.idx_container = clamp_index(state.idx_container + 1, state.containers.len()),
                }
                reset_following(state, state.page);
            }
            KeyCode::Char('c') => {
                if state.page == Page::Container {
                    let cluster = &state.clusters[state.idx_cluster];
                    let task = &state.tasks[state.idx_task];
                    let container = &state.containers[state.idx_container];
                    if cluster.is_empty() || task.is_empty() || container.is_empty() {
                        return Ok(false);
                    }
                    ratatui::restore();
                    ecs_execute_command(cluster, task, container, "/bin/sh").await;
                    return Ok(true);
                }
            }

            KeyCode::Char('p') => {
                if state.page == Page::Container {
                    let cluster = &state.clusters[state.idx_cluster];
                    let task = &state.tasks[state.idx_task];
                    let runtime_id = &state.runtime_ids[state.idx_container];
                    if cluster.is_empty() || task.is_empty() || runtime_id.is_empty() {
                        return Ok(false);
                    }
                    ratatui::restore();
                    let host = crate::commands::port_forward::select_host(&"What host do you want to use?".to_string());
                    let remote_port = crate::commands::port_forward::select_port(&"What remote port do you want to use?".to_string());
                    let local_port = crate::commands::port_forward::select_port(&"What local port do you want to use?".to_string());
                    let target = format!("ecs:{}_{}_{}", cluster, task, runtime_id);
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
        Page::Cluster => draw_list_block(Page::Cluster.title(), &state.clusters, state.idx_cluster),
        Page::Services => draw_list_block(Page::Services.title(), &state.services, state.idx_service),
        Page::Tasks => draw_list_block(Page::Tasks.title(), &state.tasks, state.idx_task),
        Page::Container => draw_list_block(Page::Container.title(), &state.containers, state.idx_container),
    };
    frame.render_stateful_widget(list, left_area, &mut list_state);

    // right: details / selection summary as Vec<Line>
    let mut details = vec![
        Line::from(Span::raw(format!("Page: {}", state.page.title()))),
        Line::from(""),
        Line::from(Span::raw(format!("Cluster:  {}", state.clusters.get(state.idx_cluster).unwrap_or(&"None".to_string())))),
        Line::from(Span::raw(format!("Service: {}", state.services.get(state.idx_service).unwrap_or(&"None".to_string())))),
        Line::from(Span::raw(format!("Task: {}", state.tasks.get(state.idx_task).unwrap_or(&"None".to_string())))),
        Line::from(Span::raw(format!("Container: {}", state.containers.get(state.idx_container).unwrap_or(&"None".to_string())))),
        Line::from(""),
        Line::from("Use ←/→ to change page, ↑/↓ to move selection, Enter to advance, q to quit."),
    ];
    if state.containers.get(state.idx_container).is_some() {
        details.push(Line::from("Press 'c' to connect to the selected container."));
        details.push(Line::from("Press 'p' to port-forward a port from the selected container."));
    }
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
        (Page::Cluster, Page::Cluster.title()),
        (Page::Services, Page::Services.title()),
        (Page::Tasks, Page::Tasks.title()),
        (Page::Container, Page::Container.title()),
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

pub async fn run_ecs_connect(terminal: &mut ratatui::DefaultTerminal) -> std::io::Result<()> {
    // initial state - optionally load clusters here asynchronously
    let mut state = AppState::default();

    let config = aws_config::load_from_env().await;
    let client = aws_sdk_ecs::Client::new(&config);
    let clusters = get_clusters(&client).await;
    state.clusters = clusters.iter().map(|c| c.name.clone()).collect();

    loop {
        // pass the state reference into the draw closure
        terminal.draw(|frame| draw_ecs_connect(frame, &state))?;
        if handle_events(&mut state).await? {
            break Ok(());
        }
        if state.page == Page::Services && state.services.is_empty() {
            let cluster_arn = &clusters[state.idx_cluster].arn;
            let services = list_cluster_services(&client, cluster_arn).await;
            state.services = services.iter().map(|s| s.name.clone()).collect();
        } else if state.page == Page::Tasks && state.tasks.is_empty() {
            let cluster_arn = &clusters[state.idx_cluster].arn;
            let service_name = &state.services[state.idx_service];
            let tasks = list_service_tasks(&client, cluster_arn, service_name).await;
            state.tasks = tasks.iter().map(|t| t.name.clone()).collect();
        } else if state.page == Page::Container && state.containers.is_empty() {
            let cluster_arn = &clusters[state.idx_cluster].arn;
            let task_id = &state.tasks[state.idx_task];
            let containers = list_task_container(&client, cluster_arn, task_id).await;
            state.containers = containers.iter().map(|c| c.name.clone()).collect();
            state.runtime_ids = containers.iter().map(|c| c.runtime_id.clone()).collect();
        }
    }
}


pub async fn ecs_connect() {
    let mut terminal = ratatui::init();
    run_ecs_connect(&mut terminal).await.expect("TODO: Ecs connect failed");
    ratatui::restore();
}