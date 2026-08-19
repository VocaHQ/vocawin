# Cutting a Windows alpha

This is the checklist we already use. A `v*` tag is what ships a tester build. There is no second path.

## Tag

The next alpha is the next `v0.1.0-alpha.N` after the current GitHub Release. Today that is `v0.1.0-alpha.2`, so the next tag is `v0.1.0-alpha.3` unless Jatin picks another name. Tag the commit you want testers to run, then push it. That is the only trigger.

`package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json` stay at `0.1.0` across these alphas. The installers keep the `VocaWin_0.1.0_*` filenames. The Git tag is the public version. Do not bump the app version for an alpha.

Do not retag. Do not use workflow_dispatch. `.github/workflows/windows-alpha-release.yml` has no manual trigger on purpose, so a branch click cannot publish.

## What CI does

A `v*` tag push runs `windows-alpha-release.yml`. tauri-action builds unsigned NSIS and MSI and attaches them to a GitHub Release named `VocaWin <tag> (Windows alpha)`. The Release is always a prerelease. `includeUpdaterJson` stays off. There is no store signature and no auto-update.

`windows-ci.yml` on main still uploads a workflow artifact. Testers should ignore that and use the GitHub Release.

tauri-action writes a generic body about vocawin.com and SmartScreen. If you drafted real notes before the job finished, they get overwritten. Put the notes and the Ready screenshot back after the installers land.

## Release notes

Paste short, honest notes. What changed. Hold Right Alt. Audio stays on this PC. Unsigned. Windows will say the publisher is unknown. More info, then Run anyway. [vocawin.com](https://vocawin.com) points here. NSIS is current-user. MSI is the wizard. Use one. File an issue if it breaks.

Upload a real Ready-screen shot as a release asset (alpha.2 used `vocawin-ready.png`) and embed it in the body:

```md
![VocaWin Ready](https://github.com/VocaHQ/vocawin/releases/download/<tag>/vocawin-ready.png)
```

You can rename the Release to drop the `v` and the `(Windows alpha)` suffix. That is what we did on alpha.2.

## Public pages

[vocawin.com](https://vocawin.com) lives in `web/` and publishes from `main` when `web/` changes. Download buttons already go to [Releases](https://github.com/VocaHQ/vocawin/releases). Do not pin a tag in the hero, the FAQ, or JSON-LD. After this checklist landed, the site should not name a specific tag at all. Check the live page still says developer alpha, unsigned, More info then Run anyway, and that every download still hits `/releases`.

The README badge is `github/v/release` with `include_prereleases`. It tracks the latest prerelease by itself. Leave it pointed at `/releases`. Do not pin a tag in the README.

`docs/setup.md` already points at `/releases`. Leave it that way.

Glance at the GitHub repo description. It should say developer alpha and GitHub Releases, not Coming soon.

The OG source under `web/assets/og/src` still says Coming soon. That is design art, not a tag you rewrite on every cut. Leave it unless VocaDesign replaces it.

## VocaHQ

VocaHQ owns vocahq.com and the family PRODUCT.md. If that page still lists an old Windows tag or still says Coming soon, ping HQ. Do not edit the HQ repo from here.

## What not to do

Do not pin a tag in the README or on vocawin.com. Do not bump the `0.1.0` app version for an alpha. Do not tell testers the build is signed, Coming soon, or Available now. Do not cut a release from a branch click.
