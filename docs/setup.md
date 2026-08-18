# Setup guide

This is the tester page for the VocaWin alpha. [vocawin.com](https://vocawin.com) points testers at [GitHub Releases](https://github.com/VocaHQ/vocawin/releases). There is no Voca account and no hosted speech API.

CI self-signs the NSIS and MSI when the VocaHQ secrets are present. That is not a store signature. SmartScreen can still warn. See [Self-signed alpha](#self-signed-alpha) if you want this PC to trust the publisher. If you skip that, use More info, then Run anyway, only if you trust the GitHub Release you downloaded.

There are two installers. The NSIS `.exe` is a current-user setup. The MSI is the WiX wizard. Use one, not both, on the same PC.

The first run asks you to download a speech-to-text model. That uses the network once. After that, audio stays on this PC.

Hold Right Alt to dictate, the same hold-default as VocaLinux. AltGr is left alone. You can change the hotkey later in Settings.

The tray mic is teal when idle, red while you speak, and amber while a take is processing.

The window title and the sidebar pill say Alpha. That means this is a tester build, not a store ship.

Logs live in the app. Open View Logs from the tray. They are this session's lines, not a file on disk. Settings and models sit under `%APPDATA%\com.vocahq.vocawin`.

## Self-signed alpha

CI signs the installers with a VocaHQ self-signed cert, `CN=VocaWin Alpha (self-signed)`, `O=VocaHQ`. That puts a publisher name on the file. It is not a purchased CA cert. [vocawin.com](https://vocawin.com) still points testers at [GitHub Releases](https://github.com/VocaHQ/vocawin/releases).

Windows will still say the publisher is unknown unless you trust [docs/certs/vocawin-alpha.cer](certs/vocawin-alpha.cer). Use Local Machine or Current User, Trusted Root or Trusted Publishers. Trusted Publishers is enough for Authenticode. SmartScreen may still warn on other people's PCs.

To trust it on Windows 10 or 11:

1. Double-click `docs/certs/vocawin-alpha.cer`.
2. Click Install Certificate.
3. Choose Current User, then Next.
4. Choose Place all certificates in the following store, Browse, Trusted Publishers, OK, Next, Finish.

SHA-256 fingerprint: `CF:7E:21:04:A2:74:5D:B2:5B:C8:65:33:0E:EA:3F:73:A1:71:6A:D6:9F:49:B6:79:53:D7:83:D2:09:7E:91:6F`. Valid until 2028-11-20.

Forks without the signing secrets still produce unsigned installers. Those builds have no publisher name.
