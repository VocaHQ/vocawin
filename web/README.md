# vocawin.com

Static landing page for VocaWin. No build step, no trackers, system fonts. The page points testers at the unsigned developer alpha on GitHub Releases.

```bash
python3 -m http.server 4173 --directory .
node --test tests/site.test.mjs
```

GitHub Pages deploys this directory from `main`.
