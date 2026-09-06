# Setup guide

This is the tester page for the VocaWin beta. [vocawin.com](https://vocawin.com) points testers at [GitHub Releases](https://github.com/VocaHQ/vocawin/releases). There is no Voca account and no hosted speech API.

The latest tagged Release is the last cut we named. If you want today's `main`, use the [nightly](https://github.com/VocaHQ/vocawin/releases/tag/nightly). Same unsigned NSIS (MSI paused while the app version is X.Y.Z-beta). Say it is a nightly and include the commit from that Release if you file an issue.

The NSIS installer is unsigned. That is not a store signature. Windows will likely say the publisher is unknown. That is SmartScreen. Use More info, then Run anyway, only if you trust the GitHub Release you downloaded.

The tagged cut ships an NSIS `.exe` (current-user). MSI is paused while the version keeps a `-beta` marker, because WiX rejects non-numeric prerelease ids.

The first run asks you to download a speech-to-text model. That uses the network once. After that, audio stays on this PC.

Hold Right Alt to dictate, the same hold-default as VocaLinux. AltGr is left alone. You can change the hotkey later in Settings.

The tray mic is teal when idle, red while you speak, amber while a take is processing, and slate when the model is unloaded or paused.

The window title and the sidebar pill say Beta. That means this is a tester build, not a store ship.

Logs live in the Debug page, also available from the tray. Warning and error show by default. Debug logging is off until you turn it on. Copy and Clear work on the in-memory buffer. Clear does not delete files on disk. Settings and models sit under `%APPDATA%\com.vocahq.vocawin`.
