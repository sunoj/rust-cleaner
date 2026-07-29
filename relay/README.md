<!--
Purpose: document one-time setup and public endpoints for the WD-40 release relay.
Exports: operator commands and publisher ownership guidance.
Deps: Cloudflare Wrangler, the wd40-release R2 bucket, scripts/release.sh.
-->
# WD-40 release relay

This dependency-free Cloudflare Worker serves the Sparkle appcast and signed WD-40 builds from R2. Run these one-time setup commands from this directory:

```bash
wrangler r2 bucket create wd40-release
wrangler deploy
wrangler secret put UPLOAD_SECRET
```

The public release URLs are:

- `https://wd40-release.sunoj-mings.workers.dev/appcast.xml`
- `https://wd40-release.sunoj-mings.workers.dev/wd40-<version>.zip`

`scripts/release.sh` is the only publisher. It uploads `appcast.xml` and signed build archives with the `UPLOAD_SECRET`; do not publish release assets manually.
