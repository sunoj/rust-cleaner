/**
 * WD-40 release relay Worker for the Sparkle appcast and signed build archives.
 * GET serves public assets; PUT stores authenticated uploads; OPTIONS enables CORS.
 * Deps: R2 binding BUCKET, secret UPLOAD_SECRET.
 */
const APPCAST_KEY = "appcast.xml";
const CORE_IDENTIFIER = "(?:0|[1-9]\\d*)";
const PRERELEASE_IDENTIFIER = "(?:0|[1-9]\\d*|[0-9A-Za-z-]*[A-Za-z-][0-9A-Za-z-]*)";
const BUILD_PATTERN = new RegExp(
  `^wd40-${CORE_IDENTIFIER}\\.${CORE_IDENTIFIER}\\.${CORE_IDENTIFIER}` +
  `(?:-${PRERELEASE_IDENTIFIER}(?:\\.${PRERELEASE_IDENTIFIER})*)?` +
  "(?:\\+[0-9A-Za-z-]+(?:\\.[0-9A-Za-z-]+)*)?\\.zip$"
);
const MIN_UPLOAD_BYTES = 10;
const MAX_UPLOAD_BYTES = 200_000_000;
const CORS = {
  "access-control-allow-origin": "*",
  "access-control-allow-methods": "GET,PUT,OPTIONS",
  "access-control-allow-headers": "authorization,content-type"
};

export default {
  async fetch(req, env) {
    if (req.method === "OPTIONS") return new Response(null, { headers: CORS });

    const key = keyFromPath(new URL(req.url).pathname);
    if (!key) return json({ error: "not found" }, 404);

    if (req.method === "GET") return serveAsset(env.BUCKET, key);
    if (req.method === "PUT") return uploadAsset(req, env, key);
    return json({ error: "not found" }, 404);
  }
};

function keyFromPath(pathname) {
  if (pathname === `/${APPCAST_KEY}`) return APPCAST_KEY;
  const key = pathname.startsWith("/") ? pathname.slice(1) : "";
  return BUILD_PATTERN.test(key) ? key : null;
}

async function serveAsset(bucket, key) {
  const obj = await bucket.get(key);
  if (!obj) return json({ error: "not found" }, 404);

  const isAppcast = key === APPCAST_KEY;
  return new Response(obj.body, {
    headers: {
      ...CORS,
      "content-type": isAppcast ? "application/xml" : "application/zip",
      "cache-control": isAppcast
        ? "public, max-age=300"
        : "public, max-age=31536000, immutable"
    }
  });
}

async function uploadAsset(req, env, key) {
  const auth = req.headers.get("authorization") || "";
  if (!env.UPLOAD_SECRET || auth !== `Bearer ${env.UPLOAD_SECRET}`) {
    return json({ error: "unauthorized" }, 401);
  }

  const declaredSize = Number(req.headers.get("content-length"));
  if (!req.body || !Number.isInteger(declaredSize) || !isValidSize(declaredSize)) {
    return json({ error: "bad size" }, 400);
  }

  await env.BUCKET.put(key, req.body, {
    httpMetadata: { contentType: key === APPCAST_KEY ? "application/xml" : "application/zip" }
  });
  return json({ ok: true, key, bytes: declaredSize });
}

function isValidSize(bytes) {
  return bytes >= MIN_UPLOAD_BYTES && bytes <= MAX_UPLOAD_BYTES;
}

function json(obj, status = 200) {
  return new Response(JSON.stringify(obj), {
    status,
    headers: { ...CORS, "content-type": "application/json" }
  });
}
