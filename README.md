# terminal-video-chat

Todos:

Client:
- [x] Make video panes use .inner() of video_area border
- [x] Make chat history list selectable and scrollable
- [ ] Visually show cursor in input box (when focused only in future)
- [ ] Let mouse scroll work as Up/Down, maybe allow clicking?
- [ ] Let users toggle focused window, Up Down should be same keys for every screen, focused screen is what action is done to.
- [ ] Make messages show as pending on client side (maybe greyed out), and update with info sent from server after
- [ ] Make chat input drop to next line if first line is full, maybe allow it to take up a \% of chat area, either way make it scrollable like chat history
- [ ] Optimize video frame => terminal pixel algorithm for speed & double vertical resolution using half-block chars (potentially dynamically change sampling resolution to achieve desired FPS) (ideally pass around max resolution that server accepts if possible, but downsample on client as necessary)

Server:
- [ ] Send ReturnToSender response to message originator with updated information of what other users received.
- [ ] Refactor common TCP util functions into common place
- [ ] Create chat rooms, with shareable names and passcodes instead of everyone connecting to the same room (maybe keep the general room for fun?)
- [ ] Add support for sending video frames over TCP socket
- [ ] Enable server to support 20-50 users in one chat room with video at once (clients only need to render one screen of video at a time), look into higher powered AWS server/load balanced server instances

Graphics options:
- Color in background, gives us rectangle pixels
- Use half-blocks like viu -b, gives us blurry but decent picture, can try to get viuer working or just copy tui-image and improve with half blocks
- Use braille pixels from Canvas library, gives us rectangle pixels but stylized
- viuer, won't be able to integrate as widget for custom kitty graphics, but could overlay on top if I figure out why my thread is breaking it
- 

Ideas for optimizing speed of video transfer:
- Have client tell server which id's/addresses it wants the latest video frame of (might be slow because it has to go client => server ===> client)
- Have server maintain a list of requested video frames from each client and only send those frames, this means it just has to go server ===> client, and only rarely does client update list on server
- Put video frames in queue to be processed by threads on client? Throw away if id/addr is not being currently shown


### My Custom Data Transfer Protocol
```
[ # of bytes of body ][ body (serialized via serde) ]
[        u64         ][              x              ]
```

Sending Protocol:
1. Serialize body of message.
2. Get the number of bytes of serialized body as a u64.
3. Send the concatenation of the number of bytes followed by the serialized body through the TCP socket.

Receiving Protocol:
1. Read 64 bits from TCP socket.
2. Use that data as the length of the body and read that number of bytes.
3. Deserialize body and interpret as common data structure.
