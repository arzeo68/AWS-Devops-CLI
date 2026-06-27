use crate::commands::aws_utils::{ecs_execute_command, force_new_deployment, get_clusters, list_cluster_services, list_service_tasks, list_task_container, set_service_desired_count, AwsCtx};
use promkit::preset::readline::Readline;
use ratatui::crossterm::event;
use ratatui::crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::{Alignment, Constraint, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, List, ListItem, ListState, Paragraph},
    Frame,
};

use ratatui::text::{Line, Span};

pub struct AwsResource {
    #[warn(dead_code)]
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
    clusters: Vec<aws_sdk_ecs::types::Cluster>,
    services: Vec<aws_sdk_ecs::types::Service>,
    tasks: Vec<String>,
    containers: Vec<String>,
    runtime_ids: Vec<String>,
    idx_cluster: usize,
    idx_service: usize,
    idx_task: usize,
    idx_container: usize,
    ctx: AwsCtx,
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
            ctx: AwsCtx::default(),
        }
    }
}

fn clamp_index(idx: usize, len: usize) -> usize {
    if len == 0 { 0 } else { idx.min(len - 1) }
}

/// Prompt for a whole number (used for scaling). Returns None if aborted/invalid.
fn prompt_i32(title: &str) -> Option<i32> {
    let mut input = Readline::default()
        .title(title)
        .validator(
            |text| text.parse::<i32>().is_ok(),
            |text| format!("Enter a whole number: {}", text),
        )
        .prompt()
        .ok()?;
    input.run().ok()?.parse::<i32>().ok()
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
                    if task.is_empty() || container.is_empty() {
                        return Ok(false);
                    }
                    ratatui::restore();
                    let tmp = cluster.cluster_arn.clone().unwrap();
                    ecs_execute_command(&state.ctx, tmp.as_str(), task, container, "/bin/sh").await;
                    return Ok(true);
                }
            }

            KeyCode::Char('p') => {
                if state.page == Page::Container {
                    let cluster = &state.clusters[state.idx_cluster];
                    let task = &state.tasks[state.idx_task];
                    let runtime_id = &state.runtime_ids[state.idx_container];
                    if task.is_empty() || runtime_id.is_empty() {
                        return Ok(false);
                    }
                    ratatui::restore();
                    let host = crate::commands::port_forward::select_host(&"What host do you want to use?".to_string());
                    let remote_port = crate::commands::port_forward::select_port(&"What remote port do you want to use?".to_string());
                    let local_port = crate::commands::port_forward::select_port(&"What local port do you want to use?".to_string());
                    let target = format!("ecs:{}_{}_{}", cluster.cluster_name.clone().unwrap(), task, runtime_id);
                    crate::commands::port_forward::connect_to_ecs_command(&state.ctx, &target, &host, &local_port, &remote_port).await;
                    return Ok(true);
                }
            }

            KeyCode::Char('r') => {
                if state.page == Page::Services {
                    let cluster = state.clusters[state.idx_cluster].cluster_arn.clone();
                    let service = state.services.get(state.idx_service).and_then(|s| s.service_name.clone());
                    if let (Some(cluster), Some(service)) = (cluster, service) {
                        ratatui::restore();
                        let config = state.ctx.config().await;
                        let client = aws_sdk_ecs::Client::new(&config);
                        if force_new_deployment(&client, &cluster, &service).await {
                            println!("Triggered force-new-deployment for service '{}'", service);
                        }
                        return Ok(true);
                    }
                }
            }

            KeyCode::Char('s') => {
                if state.page == Page::Services {
                    let cluster = state.clusters[state.idx_cluster].cluster_arn.clone();
                    let service = state.services.get(state.idx_service).and_then(|s| s.service_name.clone());
                    if let (Some(cluster), Some(service)) = (cluster, service) {
                        ratatui::restore();
                        match prompt_i32(&format!("New desired count for '{}'?", service)) {
                            Some(count) if count >= 0 => {
                                let config = state.ctx.config().await;
                                let client = aws_sdk_ecs::Client::new(&config);
                                if set_service_desired_count(&client, &cluster, &service, count).await {
                                    println!("Set desired count of '{}' to {}", service, count);
                                }
                            }
                            _ => println!("Aborted: invalid count"),
                        }
                        return Ok(true);
                    }
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
        .block(Block::bordered().title(title).title_alignment(Alignment::Center))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD).fg(Color::Yellow));
    (list, ls)
}

fn draw_ecs_connect(frame: &mut Frame, state: &AppState) {
    use Constraint::{Fill, Length};

    let vertical = Layout::vertical([Length(10),Fill(3),Length(1),Length(3)]);
    let [details_area, main_area, keyhint_area, status_area] = vertical.areas(frame.area());

    let cluster_names: Vec<String>;
    let service_names: Vec<String>;

    // left: current page list
    let (list, mut list_state) = match state.page {
        Page::Cluster => {
            cluster_names = state
                .clusters
                .iter()
                .map(|c| c.cluster_name.clone().unwrap_or_else(|| "None".to_string()))
                .collect();
            draw_list_block(Page::Cluster.title(), &cluster_names, state.idx_cluster)
        }
        Page::Services => {
            service_names = state
                .services
                .iter()
                .map(|c| c.service_name.clone().unwrap_or_else(|| "None".to_string()))
                .collect();
            draw_list_block(Page::Services.title(), &service_names, state.idx_service)
        }
        Page::Tasks => draw_list_block(Page::Tasks.title(), &state.tasks, state.idx_task),
        Page::Container => draw_list_block(Page::Container.title(), &state.containers, state.idx_container),
    };
    frame.render_stateful_widget(list, main_area, &mut list_state);

    // Top: details about the current selection


    let details_block = Block::bordered().title("Details").title_alignment(Alignment::Center);
    let inner = details_block.inner(details_area);
    frame.render_widget(details_block, details_area);

    // split inner area into three columns
    let cols = Layout::horizontal([
        Constraint::Percentage(33),
        Constraint::Percentage(33),
        Constraint::Percentage(34),
    ])
    .split(inner);

    let current_cluster = state.clusters.get(state.idx_cluster).expect("No Cluster found!");
    let current_service  = state.services.get(state.idx_service);

    // styles for key and value
    let key_style = Style::default().fg(Color::White).add_modifier(Modifier::BOLD);
    let val_style = Style::default().fg(Color::Yellow);

    // helper to build a key/value Line
    let kv_line = |key: &str, value: String| {
        Line::from(vec![
            Span::styled(format!("{}: ", key), key_style),
            Span::styled(value, val_style),
        ])
    };

    // build column contents
    let mut col1 = vec![kv_line("Cluster", current_cluster.cluster_name.clone().unwrap_or("<empty>".to_string())) ];
    let mut col2 = vec![];
    let mut col3 = vec![ Line::from("Use ←/→ to change page, ↑/↓ to move selection, Enter to advance, q to quit.") ];
    if let Some(service) = current_service {
        let task_def = service.task_definition.clone().unwrap_or("<empty>".to_string()).split("/").last().unwrap().to_string();

        col1.push(kv_line("Service", service.service_name.clone().unwrap_or("<empty>".to_string())));
        col1.push(kv_line("Status", service.status.clone().unwrap_or("<empty>".to_string())));
        col1.push(kv_line("Platform Family", service.platform_family.clone().unwrap_or("<empty>".to_string())));
        col1.push(kv_line("Platform Version", service.platform_version.clone().unwrap_or("<empty>".to_string())));
        col1.push(kv_line("Task Definition", task_def));

        col2.push(kv_line("Desired count", service.desired_count.to_string()));
        col2.push(kv_line("Running count", service.running_count.to_string()));
        col2.push(kv_line("Pending count", service.pending_count.to_string()));
        col2.push(kv_line("Created At", service.created_at.unwrap().to_string()));
        col2.push(kv_line("Execute command", service.enable_execute_command.to_string()));
        col2.push(kv_line("Managed tags", service.enable_ecs_managed_tags.to_string()));



    }

    if state.page == Page::Services && current_service.is_some() {
        col3.push(Line::from("Press 'r' to restart (force new deployment)."));
        col3.push(Line::from("Press 's' to scale the desired count."));
    }

    if state.containers.get(state.idx_container).is_some() {
        col3.push(Line::from("Press 'c' to connect to the selected container."));
        col3.push(Line::from("Press 'p' to port-forward a port from the selected container."));
    }

    // render paragraphs into the three columns
    let p1 = Paragraph::new(col1).alignment(Alignment::Left);
    let p2 = Paragraph::new(col2).alignment(Alignment::Left);
    let p3 = Paragraph::new(col3).alignment(Alignment::Left);

    frame.render_widget(p1, cols[0]);
    frame.render_widget(p2, cols[1]);
    frame.render_widget(p3, cols[2]);

    // Full-width, per-page key-hint bar (always visible, never clipped into a column)
    let hint = match state.page {
        Page::Cluster => " ↑/↓ select   Enter open   q quit",
        Page::Services => " ↑/↓ select   Enter open   [r] restart   [s] scale   ← back   q quit",
        Page::Tasks => " ↑/↓ select   Enter open   ← back   q quit",
        Page::Container => " ↑/↓ select   [c] connect   [p] port-forward   ← back   q quit",
    };
    let hint_style = Style::default()
        .fg(Color::Black)
        .bg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    frame.render_widget(Paragraph::new(Span::styled(hint, hint_style)), keyhint_area);

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

pub async fn run_ecs_connect(terminal: &mut ratatui::DefaultTerminal, ctx: AwsCtx) -> std::io::Result<()> {
    // initial state - optionally load clusters here asynchronously
    let mut state = AppState { ctx, ..Default::default() };

    let config = state.ctx.config().await;
    let client = aws_sdk_ecs::Client::new(&config);
    let clusters = get_clusters(&client).await;
    match clusters {
        None => {
            println!("No ECS clusters found");
            return Ok(());
        }
        _ => {}
    }
    state.clusters = clusters.unwrap().clusters.expect("No clusters found").clone();

    loop {
        // pass the state reference into the draw closure
        terminal.draw(|frame| draw_ecs_connect(frame, &state))?;
        if handle_events(&mut state).await? {
            break Ok(());
        }
        if state.page == Page::Services && state.services.is_empty() {
            let cluster_arn = state.clusters.get(state.idx_cluster).unwrap().clone().cluster_arn;
            let services = list_cluster_services(&client, cluster_arn.unwrap().as_str()).await;
            state.services = services
        } else if state.page == Page::Tasks && state.tasks.is_empty() {
            let cluster_arn = state.clusters.get(state.idx_cluster).unwrap().clone().cluster_arn;
            let service_name = state.services.get(state.idx_service).unwrap().clone().service_name.unwrap();
            let tasks = list_service_tasks(&client, cluster_arn.unwrap().as_str(), service_name.as_str()).await;
            state.tasks = tasks.iter().map(|t| t.name.clone()).collect();
        } else if state.page == Page::Container && state.containers.is_empty() {
            let cluster_arn = state.clusters.get(state.idx_cluster).unwrap().clone().cluster_arn;
            let task_id = &state.tasks[state.idx_task];
            let containers = list_task_container(&client, cluster_arn.unwrap().as_str(), task_id).await;
            state.containers = containers.iter().map(|c| c.name.clone()).collect();
            state.runtime_ids = containers.iter().map(|c| c.runtime_id.clone()).collect();
        }
    }
}


pub async fn ecs_connect(ctx: AwsCtx) {
    let mut terminal = ratatui::init();
    run_ecs_connect(&mut terminal, ctx).await.expect("TODO: Ecs connect failed");
    ratatui::restore();
}