; Included at the top of Tauri's installer.nsi, before the MUI pages.
; Defines here stick, so we can brand welcome/finish without forking the template.

!define MUI_WELCOMEPAGE_TITLE "VocaWin"
!define MUI_WELCOMEPAGE_TEXT "This setup puts VocaWin on this PC. Hold a hotkey, speak, and text is meant to land at your cursor. After you download a speech-to-text model, dictation stays on this computer. This build is an unsigned beta. vocawin.com is live and points testers at GitHub Releases."

!define MUI_FINISHPAGE_TITLE "VocaWin is installed."
!define MUI_FINISHPAGE_TEXT "The first run will ask you to download a model. That uses the network once. After that, audio stays on this PC."
!define MUI_FINISHPAGE_LINK "Open the setup guide"
!define MUI_FINISHPAGE_LINK_LOCATION "https://github.com/VocaHQ/vocawin/blob/main/docs/setup.md"
