use tokio::{
    io::AsyncBufReadExt,
    io::AsyncWriteExt,
    io::{AsyncReadExt, BufReader},
    net::{tcp::WriteHalf, TcpListener, TcpStream},
    sync::broadcast,
};

use chrono::prelude::*;
use crossterm::{
    event::{self, Event as CEvent, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use rand::{distributions::Alphanumeric, prelude::*};
use serde::{Deserialize, Serialize};
use std::io;
use std::io::Write;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use std::{borrow::Cow, fs};
use thiserror::Error;
use tui::widgets::{
    canvas::{Canvas, Points},
    Wrap,
};
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
use tui::{text::Text, widgets::Widget};
use viuer::{print_from_file, Config};

use image::RgbImage;
use std::collections::HashMap;
use std::path::Path;

use tui_image::{ColorMode, Image};

// use nokhwa::{Camera, CameraFormat, FrameFormat};

// #[derive(Serialize, Deserialize, Clone)]
// struct Pet {
//     id: usize,
//     name: String,
//     category: String,
//     age: usize,
//     created_at: DateTime<Utc>,
// }

enum Event {
    UserInput(crossterm::event::KeyEvent),
    ServerInput(ChatData),
    Tick,
}

// #[derive(Copy, Clone, Debug)]
// enum MenuItem {
//     Home,
//     Pets,
// }

// impl From<MenuItem> for usize {
//     fn from(input: MenuItem) -> usize {
//         match input {
//             MenuItem::Home => 0,
//             MenuItem::Pets => 1,
//         }
//     }
// }

#[derive(Serialize, Deserialize)]
pub enum ChatData {
    // HelloLan(String, u16),                     // user_name, server_port
    // HelloUser(String),                         // user_name
    ChatMessage(String), // content
                         // Video(Option<(Vec<RGB8>, usize, usize)>), // Option of (stream_data, width, height ) None means stream has ended
}

fn convert_to_stream_data(chat_data: &ChatData) -> Vec<u8> {
    let buf = bincode::serialize(chat_data).expect("serialize failed");
    let buf_with_header = [&(buf.len() as u64).to_be_bytes(), &buf[..]].concat();
    return buf_with_header;
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (tx, rx) = mpsc::channel();
    let tick_rate = Duration::from_millis(200);

    let mess = ChatData::ChatMessage(String::from("test message from client"));
    let stream = TcpStream::connect("127.0.0.1:8080").await?;
    let (reader, mut writer) = stream.into_split();
    let mut buf_reader = BufReader::new(reader);

    // let mess_data = convert_to_stream_data(&mess);
    // writer.write_all(&mess_data).await?;

    let tx1 = tx.clone();
    let _user_input_handler = thread::spawn(move || {
        let mut last_tick = Instant::now();
        loop {
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if event::poll(timeout).expect("poll works") {
                if let CEvent::Key(key) = event::read().expect("can read events") {
                    tx1.send(Event::UserInput(key)).expect("can send events");
                }
            }

            if last_tick.elapsed() >= tick_rate {
                if let Ok(_) = tx1.send(Event::Tick) {
                    last_tick = Instant::now();
                }
            }
        }
    });
    let tx2 = tx.clone();
    let _server_input_handler = tokio::spawn(async move {
        loop {
            // get incoming message from client
            let res = buf_reader.read_u64().await;
            match res {
                Ok(size) => {
                    let mut buf = vec![0u8; size as usize];
                    let res = buf_reader.read_exact(&mut buf).await;
                    match res {
                        Ok(bytes_read) if bytes_read == size as usize => {
                            let chat_data =
                                bincode::deserialize(&buf).expect("deserialize should work");
                            match chat_data {
                                ChatData::ChatMessage(msg) => {
                                    // println!("got message: {msg}");
                                    tx2.send(Event::ServerInput(ChatData::ChatMessage(msg)))
                                        .unwrap();
                                }
                                _ => {
                                    println!("dont know what chat_data is");
                                }
                            }
                        }
                        Ok(_) => {
                            println!("didn't read right number of bytes");
                        }
                        Err(e) => match e.kind() {
                            tokio::io::ErrorKind::UnexpectedEof => {
                                println!("client disconnected");
                                break;
                            }
                            _ => println!("read u64: {}", e),
                        },
                    }
                }
                Err(e) => match e.kind() {
                    tokio::io::ErrorKind::UnexpectedEof => {
                        println!("client disconnected");
                        break;
                    }
                    _ => println!("read u64: {}", e),
                },
            }
        }
    });

    enable_raw_mode().expect("can run in raw mode");
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;
    terminal.hide_cursor()?;

    let mut chat_history: Vec<String> = Vec::with_capacity(8);
    let mut current_input = String::with_capacity(16);

    let mut chat_history_prev_width = 0;
    let mut chat_history_stick_to_bottom = true;

    // let mut chat_history_line_start_index: usize = 0;
    // let mut chat_history_message_index: usize = 0;
    let mut chat_history_message_line_index: usize = 0;
    // let mut chat_history_line_offset: i64 = 0;

    // let mut chat_history_selected_ind: Option<usize> = None;
    // let mut chat_history_list_state = ListState::default();
    // chat_history_list_state.select(None);

    let mut current_chat_input_index: usize = 0;
    let mut current_chat_input_scroll_index: u16 = 0;

    // println!("start get colors");
    let img_path = "test_image.jpg";
    // let img = open(img_path);
    // let width = img.width();
    // let height = img.height();
    // let img_data = group_by_color(img);
    // println!("finished getting colors");

    let img = image::open(img_path)?.to_rgba8();

    // loop {}
    // return Ok(());
    // let mut camera = Camera::new(
    //     0,
    //     Some(CameraFormat::new_from(1920, 1080, FrameFormat::MJPEG, 30)),
    // )
    // .unwrap();
    // camera.open_stream().unwrap();
    // for i in 0..10 {
    // let frame = camera.frame().unwrap();
    // let mut file = fs::File::create(&format!("frame-{}.jpg", i)).unwrap();
    // file.write_all(frame.as_raw()).unwrap();
    // let res = frame.save("hi.jpg");
    // let tmp = frame.get_pixel(0, 0);
    // println!("{} {} {}", frame.width(), frame.height(), tmp[2]);
    // match res {
    //     Ok(file) => file,
    // Err(error) => panic!("Problem opening the file: {:?}", error),
    // }
    // frame.save_with_format(&format!("frame-{}.jpeg", i), image::ImageFormat::Jpeg).unwrap();
    // let mut file = fs::File::create(&format!("frame-{}.jpg", i)).unwrap();
    // let tmp = frame.into_raw();
    // file.write_all(&tmp).unwrap();
    // }
    // use rscam::{Camera, Config};

    // let mut camera = Camera::new("/dev/video0").unwrap();

    // camera.start(&Config {
    //     interval: (1, 30),
    //     resolution: (1920, 1080),
    //     format: b"MJPG",
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
                .constraints([Constraint::Min(20), Constraint::Percentage(20)].as_ref())
                .split(screen_size);
            let video_area = hoz_areas[0];
            let chat_area = hoz_areas[1];

            let video_frame = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::White))
                .border_type(BorderType::Double)
                .style(Style::default().bg(Color::Black));
            screen_area.render_widget(video_frame.clone(), video_area);
            let num_video_panes = 2;
            let video_panes = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(
                    [Constraint::Percentage(100 / num_video_panes)]
                        .repeat(num_video_panes.into())
                        .as_ref(),
                )
                .split(video_frame.inner(video_area));
            for video_pane in video_panes {
                let img = img.clone();
                screen_area
                    .render_widget(Image::with_img(img).color_mode(ColorMode::Rgb), video_pane);
            }

            let chat_frame = Block::default()
                .title("Chat")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::White))
                .border_type(BorderType::Double)
                .style(Style::default().bg(Color::Black));
            screen_area.render_widget(chat_frame.clone(), chat_area);

            let chat_sections = Layout::default()
                .direction(Direction::Vertical)
                // .margin(1)
                .constraints([Constraint::Min(1), Constraint::Length(1)].as_ref())
                .split(chat_frame.inner(chat_area));
            let chat_history_area = chat_sections[0];
            let chat_input_area = chat_sections[1];

            let chat_history_area_height = chat_history_area.height as usize;
            let chat_history_area_width = chat_history_area.width as usize;
            if chat_history_area_height > 0 && chat_history_area_width > 0 {
                let chat_history_lines: Vec<_> = chat_history
                    .iter()
                    .enumerate()
                    .flat_map(|(ind, chat_msg)| {
                        textwrap::wrap(chat_msg, chat_history_area_width)
                            .iter()
                            .map(|t| {
                                Spans::from(Span::styled(
                                    t.clone().into_owned(),
                                    Style::default().bg(if ind & 1 == 0 {
                                        Color::Gray
                                    } else {
                                        Color::Yellow
                                    }),
                                ))
                            })
                            .collect::<Vec<_>>()
                    })
                    .collect(); // only map as needed in future, alternate colors of messages in future

                // force chat_history_message_line_index within bounds
                if chat_history_stick_to_bottom
                    || chat_history_message_line_index + chat_history_area_height
                        > chat_history_lines.len()
                {
                    chat_history_message_line_index = chat_history_lines
                        .len()
                        .saturating_sub(chat_history_area_height);
                    chat_history_stick_to_bottom = true;
                }

                let chat_history_items: Vec<ListItem> = chat_history_lines
                    .get(
                        chat_history_message_line_index
                            ..std::cmp::min(
                                chat_history_message_line_index + chat_history_area_height,
                                chat_history_lines.len(),
                            ),
                    )
                    .unwrap()
                    .iter()
                    .map(|chat_msg| ListItem::new(chat_msg.clone()))
                    .collect();
                let chat_history_widget = List::new(chat_history_items)
                    .style(Style::default().fg(Color::Black))
                    .block(Block::default().style(Style::default().fg(Color::White)));
                screen_area.render_widget(chat_history_widget, chat_history_area);
            }

            // chat_history_line_offset = 0;
            // chat_history_prev_width = chat_history_area_width;

            // if message_ind+line_ind is as far down as possible, set chat_history_stick_to_bottom = true

            // use calculated chat_history_line_start_index to create list of up to chat_history_area_height items from chat_history_lines

            // // update visible window only if chat history is non-empty
            // if let Some(selected_ind) = chat_history_selected_ind {
            //     if chat_history_area_len > 0 {
            //         // if selected_ind is out of bounds of visible window (it will be within bounds of chat_history), move visible window to be within bounds
            //         if selected_ind >=  {

            //         } else if selected_ind < chat_history_start_index {

            //         }

            //         // update list state index to match selected_ind
            //         // chat_history_list_state.select(Some(selected_ind - chat_history_start_index));
            //     }
            // } else {
            //     // update list state index to match selected_ind
            //     chat_history_list_state.select(None);
            // }

            // // let chat_history = textwrap::wrap(
            // //     "textwrap: an efficient and powerful library for wrapping text.",
            // //     chat_history_area.width as usize,
            // // );
            // let chat_history_items: Vec<ListItem> = chat_history
            //     .iter()
            //     .map(|chat_msg| textwrap::wrap(chat_msg, chat_history_area.width as usize))
            //     .reduce(f)
            //     // .get(
            //     //     chat_history_start_index
            //     //         ..std::cmp::min(
            //     //             chat_history_start_index + chat_history_area_len,
            //     //             chat_history.len(),
            //     //         ),
            //     // )
            //     // .unwrap()
            //     // .iter()
            //     // .map(|chat_msg| ListItem::new(Text::from(vec![Spans::from(chat_msg.clone())])))
            //     // .collect();
            // let chat_history_widget = List::new(chat_history_items)
            //     .style(Style::default().fg(Color::LightCyan))
            //     .block(Block::default().style(Style::default().fg(Color::White)))
            //     .highlight_style(
            //         Style::default()
            //             .bg(Color::Yellow)
            //             .fg(Color::Black)
            //             .add_modifier(Modifier::BOLD),
            //     );
            // screen_area.render_stateful_widget(
            //     chat_history_widget,
            //     chat_history_area,
            //     &mut chat_history_list_state,
            // );

            if current_chat_input_index < current_chat_input_scroll_index as usize {
                // we need to scroll to the left
                current_chat_input_scroll_index = current_chat_input_index as u16;
            } else if current_chat_input_index
                >= (current_chat_input_scroll_index + chat_input_area.width) as usize
            {
                // we need to scroll to the right
                current_chat_input_scroll_index =
                    current_chat_input_index as u16 - chat_input_area.width + 1;
            }

            let chat_input_widget = Paragraph::new(current_input.clone())
                .block(Block::default().style(Style::default().fg(Color::Rgb(255, 150, 150))))
                .scroll((0, current_chat_input_scroll_index));

            if chat_input_area.height > 0 && chat_input_area.width > 0 {
                screen_area.set_cursor(
                    chat_input_area.x + current_chat_input_index as u16
                        - current_chat_input_scroll_index,
                    chat_input_area.y,
                );
            }

            screen_area.render_widget(chat_input_widget, chat_input_area);
        })?;

        match rx.recv()? {
            Event::UserInput(event) => match event.code {
                // check for ctrl vs no ctrl modifier, ctrl-C/D should quit also
                // ctrl-Q/ESC should quit to main menu once we have rooms setup
                KeyCode::Char('q') => {
                    let mut stdout = io::stdout();
                    execute!(stdout, LeaveAlternateScreen)?;
                    disable_raw_mode()?;
                    terminal.show_cursor()?;
                    // once meeting rooms are setup, figure out how we want to quit threads etc.
                    break;
                }
                KeyCode::Char(c) => {
                    // create helper function to, when pushing a char (unless in insert mode), always move cursor one right
                    current_input.insert(current_chat_input_index, c);
                    current_chat_input_index += 1;
                }
                KeyCode::Backspace => {
                    if current_chat_input_index > 0 {
                        current_input.remove(current_chat_input_index - 1);
                        current_chat_input_index -= 1;
                    }
                }
                KeyCode::Delete => {
                    if current_chat_input_index < current_input.len() {
                        current_input.remove(current_chat_input_index);
                    }
                }
                KeyCode::Enter => {
                    let user_message = current_input.clone();
                    // send to server
                    let mess_data =
                        convert_to_stream_data(&ChatData::ChatMessage(user_message.clone()));
                    writer.write_all(&mess_data).await?;
                    // initial add to chat history (will update after server response)
                    // chat_history.push(user_message);
                    current_input.clear();
                    current_chat_input_index = 0;
                }
                KeyCode::Up => {
                    // chat_history_message_line_index -= 1;
                    if chat_history_message_line_index > 0 {
                        chat_history_message_line_index -= 1;
                    }
                    chat_history_stick_to_bottom = false;
                    // if let Some(selected_ind) = chat_history_selected_ind {
                    //     if selected_ind > 0 {
                    //         chat_history_selected_ind = Some(selected_ind - 1);
                    //         // since terminal.draw has access to space available for chat history, we do the logic for changing chat_history_start_index there
                    //     }
                    // } else if chat_history.len() > 0 {
                    //     // should never be needed since when a new chat message is added we update selected_ind
                    //     chat_history_selected_ind = Some(0);
                    // }
                }
                KeyCode::Down => {
                    chat_history_message_line_index += 1;
                    // let UI keep within bounds
                    // chat_history_message_line_index += 1;
                    // if let Some(selected_ind) = chat_history_selected_ind {
                    //     if selected_ind + 1 < chat_history.len() {
                    //         chat_history_selected_ind = Some(selected_ind + 1);
                    //         // since terminal.draw has access to space available for chat history, we do the logic for changing chat_history_start_index there
                    //     }
                    // } else if chat_history.len() > 0 {
                    //     // should never be needed since when a new chat message is added we update selected_ind
                    //     chat_history_selected_ind = Some(0);
                    // }
                }
                KeyCode::Right => {
                    // move input cursor right (if chat input is focoused)
                    if current_chat_input_index < current_input.len() {
                        current_chat_input_index += 1;
                    }
                }
                KeyCode::Left => {
                    // move input cursor left (if chat input is focused)
                    if current_chat_input_index > 0 {
                        current_chat_input_index -= 1;
                    }
                }
                _ => {}
            },
            Event::ServerInput(chat_data) => match chat_data {
                ChatData::ChatMessage(chat_message) => {
                    // make this a helper function so we don't forget to update selected_ind
                    chat_history.push(chat_message);
                    // // update selected_ind
                    // if let Some(selected_ind) = chat_history_selected_ind {
                    //     // keep selected_ind at bottom if user was already at bottom of chat history
                    //     if selected_ind == chat_history.len() - 2 {
                    //         chat_history_selected_ind = Some(chat_history.len() - 1);
                    //     }
                    // } else {
                    //     // can put in helper function to share with failsafe, init_selected_ind
                    //     chat_history_selected_ind = Some(0);
                    // }
                }
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

    // user_input_handler.join().expect("The user input thread has panicked");
    // server_input_handler.await.expect("The server input thread has panicked");
    return Ok(());
}

pub fn open<P>(path: P) -> RgbImage
where
    P: AsRef<Path>,
{
    let img = image::open(path).unwrap();
    img.to_rgb8()
}

// fn chat_history_move_by(
//     line_offset: i64,
//     lines: &Vec<Vec<Cow<str>>>,
//     message_ind: usize,
//     line_ind: usize,
//     area_height: usize,
// ) -> (usize, usize) {
//     let mut lines_passed = 0;
//     if line_offset > 0 {
//         let lines_after = lines
//             .get(message_ind..)
//             .unwrap()
//             .iter()
//             .flatten()
//             .collect::<Vec<_>>()
//             .len();
//         for (ind, lines) in lines.get(message_ind..).unwrap().iter().enumerate() {
//             lines_passed += lines.len();
//             if lines_passed >= line_offset {
//                 return (ind, (lines.len() - 1) - (lines_passed - line_offset));
//             } else if lines_after - lines_passed < area_height {
//                 return (ind, (lines_after - lines_passed));
//             }
//         }
//     } else if line_offset < 0 {
//     }
// }
