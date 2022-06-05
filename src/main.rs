use chrono::prelude::*;
use crossterm::{
    event::{self, Event as CEvent, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use rand::{distributions::Alphanumeric, prelude::*};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::io;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use thiserror::Error;
use tui::widgets::canvas::{Canvas, Points};
use tui::widgets::Widget;
use tui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{
        Block, BorderType, Borders, Cell, List, ListItem, ListState, Paragraph, Row, Table, Tabs,
    },
    Terminal,
};
use viuer::{print_from_file, Config};

use image::RgbImage;
use std::collections::HashMap;
use std::path::Path;

use tui_image::{ColorMode, Image};

use nokhwa::{Camera, CameraFormat, FrameFormat};

const DB_PATH: &str = "./data/db.json";

#[derive(Serialize, Deserialize, Clone)]
struct Pet {
    id: usize,
    name: String,
    category: String,
    age: usize,
    created_at: DateTime<Utc>,
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("error reading the DB file: {0}")]
    ReadDBError(#[from] io::Error),
    #[error("error parsing the DB file: {0}")]
    ParseDBError(#[from] serde_json::Error),
}

enum Event<I> {
    Input(I),
    Tick,
}

#[derive(Copy, Clone, Debug)]
enum MenuItem {
    Home,
    Pets,
}

impl From<MenuItem> for usize {
    fn from(input: MenuItem) -> usize {
        match input {
            MenuItem::Home => 0,
            MenuItem::Pets => 1,
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (tx, rx) = mpsc::channel();
    let tick_rate = Duration::from_millis(200);
    thread::spawn(move || {
        let mut last_tick = Instant::now();
        loop {
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if event::poll(timeout).expect("poll works") {
                if let CEvent::Key(key) = event::read().expect("can read events") {
                    tx.send(Event::Input(key)).expect("can send events");
                }
            }

            if last_tick.elapsed() >= tick_rate {
                if let Ok(_) = tx.send(Event::Tick) {
                    last_tick = Instant::now();
                }
            }
        }
    });

    enable_raw_mode().expect("can run in raw mode");
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let menu_titles = vec!["Home", "Pets", "Add", "Delete", "Quit"];
    let mut active_menu_item = MenuItem::Home;

    let mut pet_list_state = ListState::default();
    pet_list_state.select(Some(0));

    // println!("start get colors");
    let img_path = "/home/justin/Pictures/image_67214593.JPG";
    // let img = open(img_path);
    // let width = img.width();
    // let height = img.height();
    // let img_data = group_by_color(img);
    // println!("finished getting colors");

    let img = image::open(img_path)?.to_rgba8();

    // let mut camera = Camera::new(
    //     0,
    //     None
    //     // Some(CameraFormat::new_from(640, 480, FrameFormat::YUYV, 30)),
    // )
    // .unwrap();
    // camera.open_stream().unwrap();
    // loop {
    //     let frame = camera.frame().unwrap();
    //     println!("{}, {}", frame.width(), frame.height());
    // }
    // use rscam::{Camera, Config};

    // let mut camera = Camera::new("/dev/video0").unwrap();

    // camera.start(&Config {
    //     // interval: (1, 30),
    //     // resolution: (1280, 720),
    //     // format: b"MJPG",
    //     ..Default::default()
    // }).unwrap();

    // for i in 0..10 {
    //     let frame = camera.capture().unwrap();
    //     let mut file = fs::File::create(&format!("frame-{}.jpg", i)).unwrap();
    //     file.write_all(&frame[..]).unwrap();
    // }
    // return Ok(());
    loop {
        terminal.draw(|screen_area| {
            let screen_size = screen_area.size();
            let hoz_areas = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([Constraint::Min(20), Constraint::Percentage(10)].as_ref())
                .split(screen_size);
            let video_area = hoz_areas[0];
            let chat_area = hoz_areas[1];
            let num_video_panes = 2;
            let video_panes = Layout::default()
                .direction(Direction::Horizontal)
                .margin(1)
                .constraints(
                    [Constraint::Percentage(100 / num_video_panes)]
                        .repeat(num_video_panes.into())
                        .as_ref(),
                )
                .split(video_area);
            let chat_sections = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(20), Constraint::Percentage(20)].as_ref())
                .split(chat_area);
            let chat_display_area = chat_sections[0];
            let chat_input_area = chat_sections[1];

            let video_frame = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::White))
                .border_type(BorderType::Double)
                .style(Style::default().bg(Color::Black));
            screen_area.render_widget(video_frame, video_area);
            for video_pane in video_panes {
                let img = img.clone();
                screen_area.render_widget(
                    Image::with_img(img).color_mode(ColorMode::Rgb),
                    video_pane
                );
            }

            let events = vec![ListItem::new(vec![Spans::from("Hello")])];
            let chat_input = List::new(events)
                .style(Style::default().fg(Color::LightCyan))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .style(Style::default().fg(Color::White))
                        .title("Chat")
                        .border_type(BorderType::Rounded),
                );
            screen_area.render_widget(chat_input, chat_display_area);
            let events2 = vec![ListItem::new(vec![Spans::from("Hello")])];
            let chat_input = List::new(events2)
                .style(Style::default().fg(Color::LightCyan))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .style(Style::default().fg(Color::White))
                        .title("Chat")
                        .border_type(BorderType::Rounded),
                );
            screen_area.render_widget(chat_input, chat_display_area);

            // let menu = menu_titles
            //     .iter()
            //     .map(|t| {
            //         let (first, rest) = t.split_at(1);
            //         Spans::from(vec![
            //             Span::styled(
            //                 first,
            //                 Style::default()
            //                     .fg(Color::Yellow)
            //                     .add_modifier(Modifier::UNDERLINED),
            //             ),
            //             Span::styled(rest, Style::default().fg(Color::White)),
            //         ])
            //     })
            //     .collect();

            // let tabs = Tabs::new(menu)
            //     .select(active_menu_item.into())
            //     .block(Block::default().title("Menu").borders(Borders::ALL))
            //     .style(Style::default().fg(Color::White))
            //     .highlight_style(Style::default().fg(Color::Yellow))
            //     .divider(Span::raw("|"));

            // rect.render_widget(tabs, chunks[0]);

            // match active_menu_item {
            //     MenuItem::Home => rect.render_widget(render_home(), chunks[1]),
            //     MenuItem::Pets => {
            //         let pets_chunks = Layout::default()
            //             .direction(Direction::Horizontal)
            //             .constraints(
            //                 [Constraint::Percentage(50), Constraint::Percentage(50)].as_ref(),
            //             )
            //             .split(chunks[1]);
            //         let (left, right) = render_pets(&pet_list_state);

            //         // let chunks = Layout::default()
            //         //     .constraints([Constraint::Percentage(100)].as_ref())
            //         //     .split(pets_chunks[1]);
            //         // println!("start calc canvas");
            //         // let canvas = Canvas::default()
            //         //     .block(
            //         //         Block::default()
            //         //             .title(format!("{}x{}", width, height))
            //         //             .borders(Borders::NONE),
            //         //     )
            //         //     .x_bounds([0.0, (width - 1) as f64])
            //         //     .y_bounds([0.0, (height - 1) as f64])
            //         //     .paint(|ctx| {
            //         //         for color in img_data.keys() {
            //         //             if let Some(points) = img_data.get(&color) {
            //         //                 ctx.draw(&Points {
            //         //                     coords: points,
            //         //                     color: Color::Rgb(color.0, color.1, color.2),
            //         //                 })
            //         //             }
            //         //         }
            //         //     });
            //         // println!("calcuated canvas");

            //         rect.render_widget(
            //             Image::with_img(img).color_mode(ColorMode::Rgb),
            //             pets_chunks[1]
            //         );

            //             // rect.render_widget(canvas, chunks[0]);
            //         // canvas.render(&mut rect, chunks[1]);
            //         rect.render_stateful_widget(left, pets_chunks[0], &mut pet_list_state);
            //         // rect.render_widget(canvas, pets_chunks[1]);

            //         // let conf = Config {
            //         //     // set offset
            //         //     x: 20,
            //         //     y: 4,
            //         //     // set dimensions
            //         //     width: Some(80),
            //         //     height: Some(25),
            //         //     ..Default::default()
            //         // };
            //         // println!("before print");
            //         // print_from_file("~/Pictures/image_67200257.JPG", &conf).expect("Image printing failed.");
            //         // println!("after print");
            //     }
            // }

            // let copyright = Paragraph::new("pet-CLI 2020 - all rights reserved")
            //     .style(Style::default().fg(Color::LightCyan))
            //     .alignment(Alignment::Center)
            //     .block(
            //         Block::default()
            //             .borders(Borders::ALL)
            //             .style(Style::default().fg(Color::White))
            //             .title("Copyright")
            //             .border_type(BorderType::Plain),
            //     );

            // rect.render_widget(copyright, chunks[2]);
        })?;

        // println!("hiIIIIIIIII");
        match rx.recv()? {
            Event::Input(event) => match event.code {
                KeyCode::Char('q') => {
                    let mut stdout = io::stdout();
                    execute!(stdout, LeaveAlternateScreen)?;
                    disable_raw_mode()?;
                    terminal.show_cursor()?;
                    break;
                }
                KeyCode::Char('h') => active_menu_item = MenuItem::Home,
                KeyCode::Char('p') => active_menu_item = MenuItem::Pets,
                KeyCode::Char('a') => {
                    add_random_pet_to_db().expect("can add new random pet");
                }
                KeyCode::Char('d') => {
                    remove_pet_at_index(&mut pet_list_state).expect("can remove pet");
                }
                KeyCode::Down => {
                    if let Some(selected) = pet_list_state.selected() {
                        let amount_pets = read_db().expect("can fetch pet list").len();
                        if selected >= amount_pets - 1 {
                            pet_list_state.select(Some(0));
                        } else {
                            pet_list_state.select(Some(selected + 1));
                        }
                    }
                }
                KeyCode::Up => {
                    if let Some(selected) = pet_list_state.selected() {
                        let amount_pets = read_db().expect("can fetch pet list").len();
                        if selected > 0 {
                            pet_list_state.select(Some(selected - 1));
                        } else {
                            pet_list_state.select(Some(amount_pets - 1));
                        }
                    }
                }
                _ => {}
            },
            Event::Tick => {}
        }
        // break;
        // thread::sleep(Duration::from_micros(15000));
    }
    // loop{}
    // println!("byeeeeeeeeeeeee");

    // let conf = Config {
    //     // set offset
    //     x: 20,
    //     y: 4,
    //     // set dimensions
    //     width: Some(80),
    //     height: Some(25),
    //     ..Default::default()
    // };
    // print_from_file("/home/justin/Pictures/image_67200257.JPG", &conf).expect("Image printing failed.");
    // println!("whhyyyyyyyyyyyyyyyy");
    return Ok(());
}

pub fn open<P>(path: P) -> RgbImage
where
    P: AsRef<Path>,
{
    let img = image::open(path).unwrap();
    img.to_rgb8()
}

pub fn group_by_color(img: RgbImage) -> HashMap<(u8, u8, u8), Vec<(f64, f64)>> {
    let mut result = HashMap::<(u8, u8, u8), Vec<(f64, f64)>>::new();
    let (_, height) = img.dimensions();
    let height = height as i32;
    for (x, y, color) in img.enumerate_pixels() {
        let x = f64::from(x);
        let y = f64::from(height - 1 - (y as i32));
        let key = (color[0], color[1], color[2]);
        if let Some(origin_value) = result.get(&key) {
            let mut value = origin_value.clone();
            value.push((x, y));
            result.insert(key, value);
        } else {
            let mut value = Vec::<(f64, f64)>::new();
            value.push((x, y));
            result.insert(key, value);
        }
    }
    result
}

fn render_home<'a>() -> Paragraph<'a> {
    let home = Paragraph::new(vec![
        Spans::from(vec![Span::raw("")]),
        Spans::from(vec![Span::raw("Welcome")]),
        Spans::from(vec![Span::raw("")]),
        Spans::from(vec![Span::raw("to")]),
        Spans::from(vec![Span::raw("")]),
        Spans::from(vec![Span::styled(
            "pet-CLI",
            Style::default().fg(Color::LightBlue),
        )]),
        Spans::from(vec![Span::raw("")]),
        Spans::from(vec![Span::raw("Press 'p' to access pets, 'a' to add random new pets and 'd' to delete the currently selected pet.")]),
    ])
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White))
            .title("Home")
            .border_type(BorderType::Plain),
    );
    return home;
}

fn read_db() -> Result<Vec<Pet>, Error> {
    let db_content = fs::read_to_string(DB_PATH)?;
    let parsed: Vec<Pet> = serde_json::from_str(&db_content)?;
    return Ok(parsed);
}

fn render_pets<'a>(pet_list_state: &ListState) -> (List<'a>, Table<'a>) {
    let pets = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::White))
        .title("Pets")
        .border_type(BorderType::Plain);

    let pet_list = read_db().expect("can fetch pet list");
    let items: Vec<_> = pet_list
        .iter()
        .map(|pet| {
            ListItem::new(Spans::from(vec![Span::styled(
                pet.name.clone(),
                Style::default(),
            )]))
        })
        .collect();

    let selected_pet = pet_list
        .get(
            pet_list_state
                .selected()
                .expect("there is always a selected pet"),
        )
        .expect("exists")
        .clone();

    let list = List::new(items).block(pets).highlight_style(
        Style::default()
            .bg(Color::Yellow)
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD),
    );

    let pet_detail = Table::new(vec![Row::new(vec![
        Cell::from(Span::raw(selected_pet.id.to_string())),
        Cell::from(Span::raw(selected_pet.name)),
        Cell::from(Span::raw(selected_pet.category)),
        Cell::from(Span::raw(selected_pet.age.to_string())),
        Cell::from(Span::raw(selected_pet.created_at.to_string())),
    ])])
    .header(Row::new(vec![
        Cell::from(Span::styled(
            "ID",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Cell::from(Span::styled(
            "Name",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Cell::from(Span::styled(
            "Category",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Cell::from(Span::styled(
            "Age",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Cell::from(Span::styled(
            "Created At",
            Style::default().add_modifier(Modifier::BOLD),
        )),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White))
            .title("Detail")
            .border_type(BorderType::Plain),
    )
    .widths(&[
        Constraint::Percentage(5),
        Constraint::Percentage(20),
        Constraint::Percentage(20),
        Constraint::Percentage(5),
        Constraint::Percentage(20),
    ]);

    return (list, pet_detail);
}

fn add_random_pet_to_db() -> Result<Vec<Pet>, Error> {
    let mut rng = rand::thread_rng();
    let db_content = fs::read_to_string(DB_PATH)?;
    let mut parsed: Vec<Pet> = serde_json::from_str(&db_content)?;
    let catsdogs = match rng.gen_range(0, 1) {
        0 => "cats",
        _ => "dogs",
    };

    let random_pet = Pet {
        id: rng.gen_range(0, 9999999),
        name: rng.sample_iter(Alphanumeric).take(10).collect(),
        category: catsdogs.to_owned(),
        age: rng.gen_range(1, 15),
        created_at: Utc::now(),
    };

    parsed.push(random_pet);
    fs::write(DB_PATH, &serde_json::to_vec(&parsed)?)?;
    Ok(parsed)
}

fn remove_pet_at_index(pet_list_state: &mut ListState) -> Result<(), Error> {
    if let Some(selected) = pet_list_state.selected() {
        let db_content = fs::read_to_string(DB_PATH)?;
        let mut parsed: Vec<Pet> = serde_json::from_str(&db_content)?;
        parsed.remove(selected);
        fs::write(DB_PATH, &serde_json::to_vec(&parsed)?)?;
        pet_list_state.select(Some(selected - 1));
    }
    Ok(())
}
