# Cutting a Windows beta

This is the checklist we already use. A `v*` tag is what ships a **named** tester cut. Nightlies are a second path: `.github/workflows/nightly.yml` publishes a moving `nightly` prerelease from `main` when app source changed. Do not use a `v*` tag for a nightly. Do not retag `nightly` by hand unless you are replacing a broken publish.

## Tag

The next named tester tag is `v0.1.0-beta.1`. Cut it only after Jatin asks. The latest tagged Release today is still `v0.1.0-alpha.3`. Do not treat `v0.1.0-beta.1` as live until that tag is pushed. Tag the commit you want testers to run, then push it. That is the only trigger.

`package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json` stay at `0.1.0` across these cuts. The installers keep the `VocaWin_0.1.0_*` filenames. The Git tag is the public version. Do not bump the app version for a beta.

Do not retag. Do not use workflow_dispatch. `.github/workflows/windows-alpha-release.yml` has no manual trigger on purpose, so a branch click cannot publish.

## What CI does

A `v*` tag push runs `windows-alpha-release.yml`. tauri-action builds unsigned NSIS and MSI and attaches them to a GitHub Release named `VocaWin <tag> (Windows beta)`. The Release is always a prerelease. `includeUpdaterJson` stays off. There is no store signature and no auto-update.

`windows-ci.yml` on main still uploads a workflow artifact. Testers should ignore that and use a GitHub Release: the latest tagged `v*` cut, or [nightly](https://github.com/VocaHQ/vocawin/releases/tag/nightly) if they want today's `main`.

The nightly Release is always a prerelease named `nightly`. It is deleted and recreated; the URL stays the same. The README release badge uses `sort=semver` so `nightly` does not steal it from `v*`.

tauri-action writes a generic body about vocawin.com and SmartScreen. If you drafted real notes before the job finished, they get overwritten. Put the notes and the Ready screenshot back after the installers land.

## Release notes

Paste short, honest notes. What changed. Hold Right Alt. Audio stays on this PC. Unsigned. Windows will say the publisher is unknown. More info, then Run anyway. [vocawin.com](https://vocawin.com) points here. NSIS is current-user. MSI is the wizard. Use one. File an issue if it breaks.

Upload a real Ready-screen shot as a release asset (alpha.2 used `vocawin-ready.png`) and embed it in the body:

```md
![VocaWin Ready](https://github.com/VocaHQ/vocawin/releases/download/<tag>/vocawin-ready.png)
```

You can rename the Release to drop the `v` and the `(Windows beta)` suffix. That is what we did on alpha.2 and alpha.3.

## Public pages

[vocawin.com](https://vocawin.com) lives in `web/` and publishes from `main` when `web/` changes. The hero download still goes to [Releases](https://github.com/VocaHQ/vocawin/releases), not a `v*` tag. A quieter [nightly](https://github.com/VocaHQ/vocawin/releases/tag/nightly) link is allowed because that tag is moving, not a version pin. Do not pin `v0.1.0-beta.1` or any other `v*` tag in the hero, the FAQ, or JSON-LD. Check the live page still says beta, unsigned, More info then Run anyway.

The README badge is `github/v/release` with `include_prereleases`. It tracks the latest prerelease by itself. Leave it pointed at `/releases`. Do not pin a tag in the README.

`docs/setup.md` already points at `/releases`. Leave it that way.

Glance at the GitHub repo description. It should say beta and GitHub Releases, not Coming soon.

The OG source under `web/assets/og/src` still says Coming soon. That is design art, not a tag you rewrite on every cut. Leave it unless VocaDesign replaces it.

## VocaHQ

VocaHQ owns vocahq.com and the family PRODUCT.md. If that page still lists an old Windows tag or still says Coming soon, ping HQ. Do not edit the HQ repo from here.

## What not to do

Do not pin a `v*` tag in the README or on vocawin.com. Do not bump the `0.1.0` app version for a beta or a nightly. Do not tell testers the build is signed, Coming soon, or Available now. Do not cut a named beta from a branch click. Nightly may be dispatched by hand.
