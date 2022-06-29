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
