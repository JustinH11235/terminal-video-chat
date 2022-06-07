use tokio::{net::TcpListener, sync::broadcast, io::BufReader, io::AsyncBufReadExt, io::AsyncWriteExt};

// impl as_bytes if needed
#[derive(Clone, Debug)]
enum MessageData {
    Chat(String),
    Image(u8),
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
            let mut line = String::new();

            loop {
                tokio::select! {
                    bytes_read = buf_reader.read_line(&mut line) => {
                        if bytes_read.expect("unrecognized chars found, handle properly") == 0 {
                            // EOF found
                            break;
                        }

                        tx.send((MessageData::Chat(line.clone()), addr)).unwrap();
                        line.clear();
                    }
                    res = rx.recv() => {
                        let (msg, incoming_addr) = res.expect("has errors for running behind, or senders dropped, handle properly");
                        
                        if addr != incoming_addr {
                            match msg {
                                MessageData::Chat(str) => {
                                    writer.write_all(str.as_bytes()).await.expect("should handle properly, just ignore, maybe send 'message failed to send' in future");
                                }
                                MessageData::Image(_image) => panic!("Image Not Implemented")
                            }
                        }
                    }
                }
            }
        });
    }
}
