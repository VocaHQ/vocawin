# Setup guide

This is the tester page for the VocaWin beta. [vocawin.com](https://vocawin.com) points testers at [GitHub Releases](https://github.com/VocaHQ/vocawin/releases). There is no Voca account and no hosted speech API.

The latest tagged Release is the last cut we named. If you want today's `main`, use the [nightly](https://github.com/VocaHQ/vocawin/releases/tag/nightly). Same unsigned NSIS (MSI paused while the app version is X.Y.Z-beta). Say it is a nightly and include the commit from that Release if you file an issue.

The NSIS installer is unsigned. That is not a store signature. Windows will likely say the publisher is unknown. That is SmartScreen. Use More info, then Run anyway, only if you trust the GitHub Release you downloaded.

The tagged cut ships an NSIS `.exe` (current-user). MSI is paused while the version keeps a `-beta` marker, because WiX rejects non-numeric prerelease ids.

The first run opens a short setup guide: pick your language, download a suggested speech-to-text model, and try a first dictation. The download uses the network once. After that, audio stays on this PC. You can run the guide again from General.

Hold Right Alt to dictate, the same hold-default as VocaLinux. AltGr is left alone. Press Escape while speaking to throw a take away. On the Shortcuts page you can change the hotkey, add a hands-free start/stop shortcut, dictate with a middle or side mouse button, or bind a shortcut that types your last dictation again. Record takes whatever you press: one side key such as Right Alt or Left Ctrl, a function key, or Ctrl or Alt (plus Shift if you like) with a letter, number, F-key, arrow, punctuation key or Space, such as Ctrl+Space. A key that types on its own needs Ctrl or Alt, since dictation holds it back while you press it. Win/Super is reserved by Windows.

A small pill shows at the bottom of the screen for a few seconds after VocaWin starts, since Windows often hides new tray icons. The same pill shows a live level while you speak and a spinner while a take is transcribed. It never takes focus from the app you are typing into. Turn either off in General.

Text is typed at the caret, a few characters at a time, and your clipboard is left alone. If an app drops or garbles typed text, add it under Formatting, Always paste in these apps (or switch How text goes in to Paste): VocaWin pastes there and puts your clipboard back afterwards. Apps running as administrator block input from VocaWin; the text is left on the clipboard instead, so press Ctrl+V.

After a take the microphone stays open, unrecorded, for 30 seconds, so the next take starts at once; Windows shows its microphone indicator until it closes. Nothing is stored or sent while it is open between takes.

Settings, Recording overlay, Show text while you speak puts what the model hears so far in the pill, about once a second, from the last 10 seconds of audio. It is off by default because it uses more CPU while you dictate; the typed text is still decoded from the whole take when you let go.

The tray mic is teal when idle, red while you speak, amber while a take is processing, and slate when the model is unloaded or paused.

The window title and the sidebar pill say Beta. That means this is a tester build, not a store ship.

Logs live in the Debug page, also available from the tray. Warning and error show by default. Debug logging is off until you turn it on. Copy and Clear work on the in-memory buffer. Clear does not delete files on disk. Settings and models sit under `%APPDATA%\com.vocahq.vocawin`. History (`history.json`), the audio of your last 50 takes (`history-audio\`), and usage stats (`stats.json`) live there too, only on this PC. History keeps 30 days by default.
