# terminal-video-chat

Graphics options:
- Color in background, gives us rectangle pixels
- Use half-blocks like viu -b, gives us blurry but decent picture, can try to get viuer working or just copy tui-image and improve with half blocks
- Use braille pixels from Canvas library, gives us rectangle pixels but stylized
- viuer, won't be able to integrate as widget for custom kitty graphics, but could overlay on top if I figure out why my thread is breaking it
- 

Todos:
- Make chat history list selectable and scrollable (is scroll bar possible?) Let users toggle focused window, Up Down should be same keys for every screen, focused screen is what action is done to.
- Make messages show as pending on client side (maybe greyed out), and update with info sent from server after
- Refactor common TCP util functions into common place
- 

### My Data Transfer Protocol
[ # of bytes of body ][ body (serialized via serde) ]
[        u64         ][              x              ]

Sending Protocol:
1. Serialize body of message.
2. Get the number of bytes of serialized body as a u64.
3. Send the concatenation of the number of bytes followed by the serialized body through the TCP socket.
Receiving Protocol:
1. Read 64 bits from TCP socket.
2. Use that data as the length of the body and read that number of bytes.
3. Deserialize body and interpret as common data structure.
