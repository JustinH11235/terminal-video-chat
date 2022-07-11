use tokio::{
    io::AsyncBufReadExt,
    io::AsyncWriteExt,
    io::{AsyncReadExt, BufReader},
    net::{tcp::WriteHalf, TcpListener, TcpStream},
    sync::broadcast,
};

use chrono::prelude::*;
use crossterm::{
    event::{self, KeyCode},
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};
use rand::{distributions::Alphanumeric, prelude::*};
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};
use std::io::Write;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use std::{borrow::Cow, fs};
use std::{fmt::Debug, io};
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

use image::{codecs::jpeg::JpegDecoder, io::Reader, ImageBuffer, RgbImage, Rgba};
use std::collections::HashMap;
use std::io::Cursor;
use std::path::Path;

use tui_image::{ColorMode, Image};

use rscam::{Camera, Config as RscamConfig, Frame};
// use nokhwa::{Camera, CameraFormat, FrameFormat};

pub mod util;

// #[derive(Serialize, Deserialize, Clone)]
// struct Pet {
//     id: usize,
//     name: String,
//     category: String,
//     age: usize,
//     created_at: DateTime<Utc>,
// }

enum Event {
    UserInputKey(crossterm::event::KeyEvent),
    UserInputFrame(Frame),
    ServerInput(ChatData),
    Tick,
}

#[derive(Serialize, Deserialize)]
pub enum ChatData {
    OtherChatMessage(String),       // message
    SelfChatMessage(String, usize), // message, id
    VideoFrame(Vec<u8>, u32, u32),  // (stream_data, width, height)
}

struct ChatMessageInfo {
    message: String,
    is_pending: bool,
    uid: usize,
    // author: num or UserStruct
}

impl fmt::Display for ChatMessageInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "User Temp: {}{}",
            self.message,
            if self.is_pending { " (pending...)" } else { "" }
        )
    }
}

impl ChatMessageInfo {
    fn new_with_uid(message: String, is_pending: bool, uid: usize) -> Self {
        return ChatMessageInfo {
            message,
            is_pending,
            uid,
        };
    }

    fn new(message: String, is_pending: bool) -> Self {
        return ChatMessageInfo {
            message,
            is_pending,
            uid: util::get_uid(),
        };
    }

    fn to_line_spans(&self, line_width: usize) -> Vec<Span> {
        return textwrap::wrap(&self.to_string(), line_width)
            .into_iter()
            .map(|message| {
                Span::styled(
                    message.into_owned(),
                    Style::default().fg(if self.is_pending {
                        Color::Gray
                    } else {
                        Color::White
                    }),
                )
            })
            .collect::<Vec<_>>();
    }
}

// make a method of ChatData
fn convert_to_stream_data(chat_data: &ChatData) -> Vec<u8> {
    let buf = bincode::serialize(chat_data).expect("serialize failed");
    let buf_with_header = [&(buf.len() as u32).to_be_bytes(), &buf[..]].concat();
    return buf_with_header;
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (tx, rx) = mpsc::channel();
    let tick_rate = Duration::from_millis(67);

    // let mess = ChatData::ChatMessage(String::from("test message from client"));
    let stream = TcpStream::connect("127.0.0.1:8080").await?;
    let (reader, mut writer) = stream.into_split();
    let mut buf_reader = BufReader::new(reader);

    // let mess_data = convert_to_stream_data(&mess);
    // writer.write_all(&mess_data).await?;

    let tx1 = tx.clone();
    let _user_input_handler = thread::spawn(move || {
        let mut camera = Camera::new("/dev/video0").unwrap();
        camera
            .start(&RscamConfig {
                interval: (1, 15),
                resolution: (176, 144),
                format: b"MJPG",
                ..Default::default()
            })
            .unwrap();
        let mut last_tick = Instant::now();
        loop {
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if event::poll(timeout).expect("poll works") {
                if let crossterm::event::Event::Key(key) = event::read().expect("can read events") {
                    tx1.send(Event::UserInputKey(key)).expect("can send events");
                }
            }

            if last_tick.elapsed() >= tick_rate {
                if let Ok(_) = tx1.send(Event::Tick) {
                    last_tick = Instant::now();
                }
            }

            let frame = camera.capture().unwrap();
            tx1.send(Event::UserInputFrame(frame))
                .expect("can send events");
            // let frame_data = convert_to_stream_data(&ChatData::VideoFrame(frame.to_vec(), , ));
            // writer.write_all(&frame_data).await?;
        }
    });
    let tx2 = tx.clone();
    let _server_input_handler = tokio::spawn(async move {
        loop {
            // get incoming message from server
            let res = buf_reader.read_u32().await;
            match res {
                Ok(size) => {
                    let mut buf = vec![0u8; size as usize];
                    let res = buf_reader.read_exact(&mut buf).await;
                    match res {
                        Ok(bytes_read) if bytes_read == size as usize => {
                            let chat_data =
                                bincode::deserialize(&buf).expect("deserialize should work");
                            tx2.send(Event::ServerInput(chat_data)).unwrap();
                            // match chat_data {
                            //     ChatData::OtherChatMessage(ref _msg) => {
                            //         // println!("got message: {msg}");
                            //         tx2.send(Event::ServerInput(chat_data)).unwrap();
                            //     }
                            //     ChatData::SelfChatMessage(ref _msg, ref _uid) => {
                            //         // println!("got message: {msg}");
                            //         tx2.send(Event::ServerInput(chat_data)).unwrap();
                            //     }
                            //     ChatData::VideoFrame(data, width, height) => tx2
                            //         .send(Event::ServerInput(ChatData::VideoFrame(
                            //             data, width, height,
                            //         )))
                            //         .unwrap(),
                            //     _ => {
                            //         println!("dont know what chat_data is");
                            //     }
                            // }
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

    let mut video_frames: Vec<ImageBuffer<Rgba<u8>, Vec<u8>>> = Vec::new();

    let mut chat_history: Vec<ChatMessageInfo> = Vec::with_capacity(8);
    let mut current_input = String::with_capacity(16);

    // let mut chat_history_prev_width = 0;
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
    video_frames.push(img);

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

            let num_video_panes: usize = 2;
            let video_panes = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(
                    [Constraint::Percentage((100 / num_video_panes) as u16)]
                        .repeat(num_video_panes.into())
                        .as_ref(),
                )
                .split(video_frame.inner(video_area));
            for (ind, video_pane) in video_panes.iter().enumerate() {
                if ind < video_frames.len() {
                    screen_area.render_widget(
                        Image::with_img(video_frames.get(ind).unwrap().to_owned())
                            .color_mode(ColorMode::Rgb),
                        *video_pane,
                    );
                }
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
                        let mut line_spans = chat_msg.to_line_spans(chat_history_area_width);
                        for span in &mut line_spans {
                            span.style = span.style.bg(if ind & 1 == 0 {
                                Color::LightMagenta
                            } else {
                                Color::LightBlue
                            });
                        }
                        return line_spans;
                    })
                    .collect(); // only map as needed in future

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
            Event::UserInputKey(event) => match event.code {
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
                    // initial add to chat history (will update after server response)
                    let chat_msg_info = ChatMessageInfo::new(user_message.clone(), true);
                    let msg_uid = chat_msg_info.uid;
                    chat_history.push(chat_msg_info);
                    // send to server
                    let mess_data =
                        convert_to_stream_data(&ChatData::SelfChatMessage(user_message, msg_uid));
                    writer.write_all(&mess_data).await?;
                    // reset input field
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
            Event::UserInputFrame(frame) => {
                let frame_data = convert_to_stream_data(&ChatData::VideoFrame(
                    frame[..].to_vec(),
                    frame.resolution.0,
                    frame.resolution.1,
                ));
                writer.write_all(&frame_data).await?;
            }
            Event::ServerInput(chat_data) => match chat_data {
                ChatData::OtherChatMessage(chat_message) => {
                    // make this a helper function so we don't forget to update selected_ind
                    chat_history.push(ChatMessageInfo::new(chat_message, false));
                    // TODO: order by timestamp
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
                ChatData::SelfChatMessage(chat_message, uid) => {
                    // this is our own message returned from server, update chat_history to display updated info
                    // remove the placeholder/pending chat message
                    chat_history.retain(|chat_msg_info| chat_msg_info.uid != uid);
                    // add up-to-update chat message to chat_history
                    chat_history.push(ChatMessageInfo::new_with_uid(chat_message, false, uid));
                }
                ChatData::VideoFrame(data, width, height) => {
                    // println!("got frame with {} {}", width, height);
                    // RgbImage::from
                    // println!("{} {}", width * height, data.len());
                    let c = Cursor::new(data.clone());
                    let r = Reader::new(c);
                    let img = r.with_guessed_format().unwrap();
                    // eprintln!("{:?}", img.format().unwrap());
                    let img = img.decode().unwrap();
                    // let temp = JpegDecoder::new(r);
                    video_frames.insert(
                        1,
                        img.to_rgba8(), // ImageBuffer::from_raw(width, height, data)
                                        //     .expect("given frame buffer is not big enough"),
                    );
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
