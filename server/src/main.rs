use serde::{Deserialize, Serialize};
use tokio::{
    io::AsyncBufReadExt,
    io::AsyncWriteExt,
    io::{AsyncReadExt, BufReader},
    net::TcpListener,
    sync::broadcast,
};

// impl as_bytes if needed
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ChatData {
    // HelloLan(String, u16),                     // user_name, server_port
    // HelloUser(String),                         // user_name
    ChatMessage(String),           // content
    VideoFrame(Vec<u8>, u32, u32), // Video(Option<(Vec<RGB8>, usize, usize)>), // Option of (stream_data, width, height ) None means stream has ended
}

fn convert_to_stream_data(chat_data: &ChatData) -> Vec<u8> {
    let buf = bincode::serialize(chat_data).expect("serialize failed");
    let buf_with_header = [&(buf.len() as u64).to_be_bytes(), &buf[..]].concat();
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
            // let mut line = String::new();

            loop {
                tokio::select! {
                    // get incoming message from client
                    res = buf_reader.read_u64() => {
                        match res {
                            Ok(size) => {
                                // println!("got data size {}", size);
                                let mut buf = vec![0u8; size as usize];
                                let res = buf_reader.read_exact(&mut buf).await;
                                match res {
                                    Ok(bytes_read) if bytes_read == size as usize => {
                                        let chat_data = bincode::deserialize(&buf).expect("deserialize should work");
                                        match chat_data {
                                            ChatData::ChatMessage(msg) => {
                                                println!("got message: \"{msg}\" from: {}", addr);
                                                tx.send((ChatData::ChatMessage(msg), addr)).unwrap();
                                            }
                                            ChatData::VideoFrame(data, width, height) => {
                                                tx.send((ChatData::VideoFrame(data, width, height), addr)).unwrap();
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
                        // if bytes_read.expect("unrecognized chars found, handle properly") == 0 {
                        //     // EOF found
                        //     break;
                        // }

                        // tx.send((ChatData::ChatMessage(line.clone()), addr)).unwrap();
                        // line.clear();
                    }
                    res = rx.recv() => {
                        let (msg, incoming_addr) = res.expect("has errors for running behind, or senders dropped, handle properly");

                        if addr != incoming_addr {
                            match msg {
                                ChatData::ChatMessage(_) => {
                                    let res = convert_to_stream_data(&msg);
                                    writer.write_all(&res).await.expect("should handle properly, just ignore, maybe send 'message failed to send' in future");
                                }
                                ChatData::VideoFrame(_, _, _) => {
                                    let res = convert_to_stream_data(&msg);
                                    writer.write_all(&res).await.expect("should handle properly, just ignore, maybe send 'message failed to send' in future");
                                }
                                // ChatData::Image(_image) => panic!("Image Not Implemented")
                            }
                        } else {
                            // special ReturnToSender response
                            match msg {
                                ChatData::ChatMessage(_) => {
                                    let res = convert_to_stream_data(&msg);
                                    writer.write_all(&res).await.expect("should handle properly, just ignore, maybe send 'message failed to send' in future");
                                }
                                ChatData::VideoFrame(_, _, _) => {
                                    let res = convert_to_stream_data(&msg);
                                    writer.write_all(&res).await.expect("should handle properly, just ignore, maybe send 'message failed to send' in future");
                                }
                                // ChatData::Image(_image) => panic!("Image Not Implemented")
                            }
                        }
                    }
                }
            }
        });
    }
}
