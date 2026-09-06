# Cutting a Windows beta

This is the checklist we already use. A `v*` tag is what ships a **named** tester cut. Nightlies are a second path: `.github/workflows/nightly.yml` publishes a moving `nightly` prerelease from `main` when app source changed. Do not use a `v*` tag for a nightly. Do not retag `nightly` by hand unless you are replacing a broken publish.

## Tag

The latest tagged tester cut is `v0.1.1-beta`. Cut the next `v*` tag only after Jatin asks. Tag the commit testers should run, then push it. That is the only trigger.

Keep the app version and the public Git tag aligned, including the beta marker. For this cut that means `0.1.1-beta` in `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`, public tag `v0.1.1-beta`, and installers named `VocaWin_0.1.1-beta_*`. Bump both together when you cut the next named release. Product status stays Beta in notes and site prose.

Do not retag. Do not use workflow_dispatch. `.github/workflows/windows-alpha-release.yml` has no manual trigger on purpose, so a branch click cannot publish.

## What CI does

A `v*` tag push runs `windows-alpha-release.yml`. tauri-action builds unsigned NSIS and MSI and attaches them to a GitHub Release named `VocaWin <tag> (Windows beta)`. For named `v*` cuts the Release is published as **Latest** (`prerelease: false`) so it appears on the repo homepage. Do not check the GitHub prerelease box for these cuts. `includeUpdaterJson` stays off. There is no store signature and no auto-update.

`windows-ci.yml` on main still uploads a workflow artifact. Testers should ignore that and use a GitHub Release: the latest tagged `v*` cut, or [nightly](https://github.com/VocaHQ/vocawin/releases/tag/nightly) if they want today's `main`.

The nightly Release is a separate path and stays a prerelease named `nightly`. It is deleted and recreated; the URL stays the same. Named `v*` cuts are Latest; nightly does not replace them on the homepage.

tauri-action writes a generic body about vocawin.com and SmartScreen. If you drafted real notes before the job finished, they get overwritten. Put the notes and the Ready screenshot back after the installers land. Keep beta language in those notes.

## Release notes

Paste short, honest notes. What changed. Hold Right Alt. Audio stays on this PC. Unsigned. Windows will say the publisher is unknown. More info, then Run anyway. [vocawin.com](https://vocawin.com) points here. NSIS is current-user. MSI is the wizard. Use one. File an issue if it breaks. Say beta in the notes.

Upload a real Ready-screen shot as a release asset (alpha.2 used `vocawin-ready.png`) and embed it in the body:

```md
![VocaWin Ready](https://github.com/VocaHQ/vocawin/releases/download/<tag>/vocawin-ready.png)
```

You can rename the Release to drop the `v` and the `(Windows beta)` suffix. That is what we did on alpha.2 and alpha.3.

## Public pages

[vocawin.com](https://vocawin.com) lives in `web/` and publishes from `main` when `web/` changes. Name the current tagged cut (`v0.1.1-beta`) next to the download facts and in JSON-LD `softwareVersion`, and link [that tag](https://github.com/VocaHQ/vocawin/releases/tag/v0.1.1-beta). The primary download button can still go to [Releases](https://github.com/VocaHQ/vocawin/releases). Nightly stays on the moving [nightly](https://github.com/VocaHQ/vocawin/releases/tag/nightly) tag. When we cut the next named tag, bump those pins in the same site PR and keep the app version aligned with the tag (including the single `-beta` marker). Check the live page still says beta, unsigned, More info then Run anyway.

The README badge is `github/v/release` with `include_prereleases`. Named `v*` cuts publish as Latest, so the homepage and badge can surface them without relying on the prerelease flag. Leave the badge pointed at `/releases`. Do not pin a tag in the README.

`docs/setup.md` already points at `/releases`. Leave it that way.

Glance at the GitHub repo description. It should say beta and GitHub Releases, not Coming soon.

The OG texture under `web/assets/og/src` still says Coming soon. That is design art, not a tag you rewrite on every cut. Leave it unless VocaDesign replaces it.

## VocaHQ

VocaHQ owns vocahq.com and the family PRODUCT.md. If that page still lists an old Windows tag or still says Coming soon, ping HQ. Do not edit the HQ repo from here.

## What not to do

Do not pin a `v*` tag in the README. vocawin.com should name the current tagged cut and get updated on the next tag. Do not ship a named cut where the app version and the `v*` tag disagree, and do not drop the `-beta` marker from either side while the cut is still beta. Do not use a numbered suffix like `-beta.1`; next cuts bump the numeric version and keep one `-beta`. Do not force the GitHub prerelease checkbox on named `v*` cuts (those publish as Latest for homepage visibility). Do not bump the app version for a nightly alone. Do not tell testers the build is signed, Coming soon, or Available now. Do not cut a named beta from a branch click. Nightly may be dispatched by hand and remains a separate prerelease path.
