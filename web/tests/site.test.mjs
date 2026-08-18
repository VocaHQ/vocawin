import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const siteRoot = fileURLToPath(new URL("..", import.meta.url));
const html = readFileSync(join(siteRoot, "index.html"), "utf8");
const css = readFileSync(join(siteRoot, "styles.css"), "utf8");
const script = readFileSync(join(siteRoot, "script.js"), "utf8");

test("page has one title and landmark structure", () => {
  assert.match(html, /<title>VocaWin: voice typing that stays on this PC<\/title>/);
  assert.equal((html.match(/<h1\b/g) || []).length, 1);
  assert.match(html, /<main id="main-content">/);
  assert.match(html, /<nav[^>]+aria-label="Main navigation"/);
  assert.match(html, /class="skip-link"/);
});

test("in-page links have matching section ids", () => {
  const anchors = [...html.matchAll(/href="#([\w-]+)"/g)].map((match) => match[1]);
  assert.ok(anchors.length > 0);
  for (const anchor of anchors) {
    assert.match(html, new RegExp(`id="${anchor}"`), `Missing #${anchor}`);
  }
});

test("document ids are unique", () => {
  const ids = [...html.matchAll(/\sid="([\w-]+)"/g)].map((match) => match[1]);
  assert.equal(new Set(ids).size, ids.length);
});

test("unsigned alpha download is explicit and not oversold", () => {
  assert.match(
    html,
    /class="hero-copy"[\s\S]*class="button button-primary" href="https:\/\/github\.com\/VocaHQ\/vocawin\/releases"/,
  );
  assert.match(html, /href="https:\/\/github\.com\/VocaHQ\/vocawin\/releases"/);
  assert.match(
    html,
    /href="https:\/\/github\.com\/VocaHQ\/vocawin\/releases\/tag\/v0\.1\.0-alpha\.1"/,
  );
  assert.match(html, /href="https:\/\/github\.com\/VocaHQ\/vocawin\/issues"/);
  assert.match(html, /href="https:\/\/github\.com\/VocaHQ\/vocawin\/blob\/main\/docs\/setup\.md"/);
  assert.match(html, /unsigned/i);
  assert.match(html, /SmartScreen|publisher is unknown/i);
  assert.match(html, /More info, then Run anyway/i);
  assert.match(html, /developer alpha/i);
  assert.match(html, /Windows · developer alpha/);
  assert.doesNotMatch(html, /href="\/setup"/);
  assert.match(html, /download the alpha/i);
  assert.match(html, /NSIS current-user setup\.exe and (an )?MSI/i);
  assert.match(html, /LimitedAvailability/);
  assert.doesNotMatch(html, /PreOrder/);
  assert.doesNotMatch(html, /no public installer/i);
  assert.doesNotMatch(html, /cannot install/i);
  assert.doesNotMatch(html, /No\.\s*There is no public installer/i);
  assert.doesNotMatch(html, /coming soon/i);
  assert.doesNotMatch(html, /100% offline/i);
  assert.doesNotMatch(html, /free forever/i);
  assert.doesNotMatch(html, /99\+\s*languages/i);
  assert.doesNotMatch(html, /AI-powered/i);
  assert.doesNotMatch(html, /googletagmanager|gtag\(|G-SHWKRJMCEN/i);
});

test("license matches PRODUCT.md AGPL-3.0-or-later", () => {
  assert.match(html, /AGPL-3\.0-or-later/);
  assert.doesNotMatch(html, /not set yet/i);
  assert.doesNotMatch(html, /License<\/b><span>Not set/i);
  assert.ok(existsSync(join(siteRoot, "..", "LICENSE")), "Missing root LICENSE");
});

test("privacy language states where processing happens", () => {
  assert.match(html, /on this Windows machine/i);
  assert.match(html, /on-device/i);
  assert.match(html, /first model download/i);
  assert.match(html, /not a Voca cloud/i);
  assert.match(html, /VocaWin does not offer a gateway mode today/i);
  assert.match(html, /Gateway mode is not on-device/i);
});

test("vocahq.com and the family are linked", () => {
  assert.match(html, /href="https:\/\/vocahq\.com\/"/);
  assert.match(html, /href="https:\/\/vocalinux\.com\/"/);
  assert.match(html, /href="https:\/\/vocamac\.com\/"/);
  assert.match(html, /href="https:\/\/vocaphone\.vocahq\.com\/"/);
  assert.match(html, /href="https:\/\/github\.com\/VocaHQ\/vocagateway"/);
  assert.match(html, /href="https:\/\/github\.com\/VocaHQ\/vocawin"/);
  assert.doesNotMatch(html, /jatinkrmalik\/vocawin/);
});

test("Windows chrome is used instead of macOS traffic lights", () => {
  assert.match(html, /class="win-captions"/);
  assert.match(css, /\.win-close::before/);
  assert.doesNotMatch(html, /traffic-lights/);
});

test("reading sections share a centered measure", () => {
  assert.match(css, /--measure:\s*1180px/);
  assert.match(css, /width:\s*min\(100%,\s*var\(--measure\)\)/);
  assert.match(css, /\.story-shell > \*,/);
});

test("manifesto window keeps the title bar full width", () => {
  assert.match(html, /class="manifesto-grid"/);
  assert.match(css, /\.privacy-grid,\s*\n\s*\.manifesto-grid\s*\{/);
  assert.doesNotMatch(css, /\.privacy-grid,\s*\n\s*\.manifesto-window\s*\{/);
});

test("all local image assets exist", () => {
  const localImages = [...html.matchAll(/(?:src|href)="((?:assets|favicon)[^"]+)"/g)].map(
    (match) => match[1],
  );
  assert.ok(localImages.includes("assets/brand/voca-logo.svg"));
  for (const asset of localImages) {
    assert.ok(existsSync(join(siteRoot, asset)), `Missing ${asset}`);
  }
});

test("production metadata is complete", () => {
  assert.match(html, /rel="canonical" href="https:\/\/vocawin\.com\/"/);
  assert.match(html, /property="og:url" content="https:\/\/vocawin\.com\/"/);
  assert.match(html, /property="og:image" content="https:\/\/vocawin\.com\/assets\/og-image\.png"/);
  assert.match(html, /name="twitter:card" content="summary_large_image"/);
  for (const asset of [
    "assets/og-image.png",
    "assets/apple-touch-icon.png",
    "assets/brand/voca-logo.svg",
    "assets/paper-dots.svg",
    "favicon.svg",
    "robots.txt",
    "sitemap.xml",
    "site.webmanifest",
  ]) {
    assert.ok(existsSync(join(siteRoot, asset)), `Missing ${asset}`);
  }
});

test("visual treatment stays flat", () => {
  const bannedFunction = ["linear-" + "gradient", "radial-" + "gradient", "conic-" + "gradient"];
  for (const token of bannedFunction) {
    assert.ok(!css.includes(token), `Unexpected ${token}`);
  }
});

test("motion has a reduced-motion fallback", () => {
  assert.match(css, /prefers-reduced-motion:\s*reduce/);
  assert.match(script, /setTimeout\([\s\S]*revealNodes[\s\S]*is-visible/);
});

test("mobile navigation can be dismissed with the keyboard", () => {
  assert.match(script, /event\.key === "Escape"/);
  assert.match(script, /closeNavigation\(\{ returnFocus: true \}\)/);
});
