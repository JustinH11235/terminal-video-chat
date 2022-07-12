use chrono::serde::ts_milliseconds;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::{
    io::AsyncBufReadExt,
    io::AsyncWriteExt,
    io::{AsyncReadExt, BufReader},
    net::TcpListener,
    sync::broadcast,
    time::sleep,
    time::Duration,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServerNetworkData {
    #[serde(with = "ts_milliseconds")]
    timestamp: DateTime<Utc>,
    // user id/struct
    chat_data: ServerChatData,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ServerChatData {
    OtherChatMessage(String),       // message
    SelfChatMessage(String, usize), // message, id
    VideoFrame(Vec<u8>, u32, u32),  // (stream_data, width, height)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ClientNetworkData {
    chat_data: ClientChatData,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ClientChatData {
    SelfChatMessage(String, usize), // message, id
    VideoFrame(Vec<u8>, u32, u32),  // (stream_data, width, height)
}

fn convert_to_stream_data(network_data: &ServerNetworkData) -> Vec<u8> {
    let buf = bincode::serialize(network_data).expect("serialize failed");
    let buf_with_header = [&(buf.len() as u32).to_be_bytes(), &buf[..]].concat();
    return buf_with_header;
}

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("localhost:8080")
        .await
        .expect("could not establish TCP connection");
    let (tx_orig, _rx) = broadcast::channel(16);

    loop {
        let (mut socket, addr) = match listener.accept().await {
            Ok((socket, addr)) => {
                println!("new client: {:?}", addr);
                (socket, addr)
            }
            Err(e) => {
                println!("couldn't get client: {:?}", e);
                continue;
            }
        };

        // if you put this above you get all the messages in the queue that came since the last client connected
        let tx = tx_orig.clone();
        let mut rx = tx.subscribe();

        tokio::spawn(async move {
            let (reader, mut writer) = socket.split();
            let mut buf_reader = BufReader::new(reader);

            loop {
                tokio::select! {
                    // get incoming message from client
                    res = buf_reader.read_u32() => {
                        match res {
                            Ok(size) => {
                                // println!("got data size {}", size);
                                let mut buf = vec![0u8; size as usize];
                                let res = buf_reader.read_exact(&mut buf).await;
                                match res {
                                    Ok(bytes_read) if bytes_read == size as usize => {
                                        let timestamp = chrono::offset::Utc::now();
                                        let data = bincode::deserialize(&buf).expect("deserialize should work");

                                        tx.send((data, addr, timestamp)).unwrap();
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
                        // if bytes_read.expect("unrecognized chars found, handle properly") == 0 {
                        //     // EOF found
                        //     break;
                        // }

                        // tx.send((ChatData::ChatMessage(line.clone()), addr)).unwrap();
                        // line.clear();
                    }
                    res = rx.recv() => {
                        let (data, incoming_addr, timestamp) = res.expect("has errors for running behind, or senders dropped, handle properly");
                            match data {
                                ClientNetworkData { chat_data: ClientChatData::SelfChatMessage(message, uid) } => {
                                    let response =
                                        if incoming_addr != addr {
                                            // from another client
                                            ServerNetworkData { timestamp: timestamp, chat_data: ServerChatData::OtherChatMessage(message) }
                                        } else {
                                            // from this client
                                            ServerNetworkData { timestamp: timestamp, chat_data: ServerChatData::SelfChatMessage(message, uid) }
                                        };
                                    let response = convert_to_stream_data(&response);
                                    writer.write_all(&response).await.expect("should handle properly, just ignore, maybe send 'message failed to send' in future");
                                }
                                ClientNetworkData { chat_data: ClientChatData::VideoFrame(data, width, height) } => {
                                    let response = ServerNetworkData { timestamp: timestamp, chat_data: ServerChatData::VideoFrame(data, width, height)};
                                    let response = convert_to_stream_data(&response);
                                    writer.write_all(&response).await.expect("should handle properly, just ignore, maybe send 'message failed to send' in future");
                                }
                                _ => {
                                    println!("Got ClientNetworkData that couldn't be recognized");
                                }
                            }

                    }
                }
            }
        });
    }
}
