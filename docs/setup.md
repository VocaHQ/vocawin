# Setup guide

This is the tester page for the unsigned VocaWin alpha. vocawin.com is still Coming soon. There is no Voca account and no hosted speech API.

Windows may warn you. SmartScreen and Unknown publisher are expected because this build is not signed. Use More info, then Run anyway, if you trust the GitHub artifact you downloaded.

There are two installers. The NSIS `.exe` is a current-user setup. The MSI is the WiX wizard. Use one, not both, on the same PC.

The first run asks you to download a speech-to-text model. That uses the network once. After that, audio stays on this PC.

Hold Right Alt to dictate, the same hold-default as VocaLinux. AltGr is left alone. You can change the hotkey later in Settings.

The tray mic is teal when idle, red while you speak, and amber while a take is processing.

The window title and the sidebar pill say Alpha. That means this is a tester build, not a store ship.

Logs live in the app. Open View Logs from the tray. They are this session's lines, not a file on disk. Settings and models sit under `%APPDATA%\com.vocahq.vocawin`.
