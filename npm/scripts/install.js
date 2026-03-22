#!/usr/bin/env node
"use strict";

const https = require("https");
const fs = require("fs");
const path = require("path");

const VERSION = "1.0.0";
const REPO = "PayEz-Net/vibesql-mail-mcp";
const BIN_DIR = path.join(__dirname, "..", "bin");

const PLATFORM_MAP = {
  "win32-x64": "vibesql-mail-mcp-windows-x64.exe",
  "linux-x64": "vibesql-mail-mcp-linux-x64",
  "darwin-x64": "vibesql-mail-mcp-macos-amd64",
  // "darwin-arm64": "vibesql-mail-mcp-macos-arm64",  // TODO: cross-compile for Apple Silicon
};

const platform = process.platform;
const arch = process.arch;
const key = `${platform}-${arch}`;
const binaryName = PLATFORM_MAP[key];

if (!binaryName) {
  console.error(`vibesql-mail-mcp: unsupported platform ${key}`);
  console.error(`Supported: ${Object.keys(PLATFORM_MAP).join(", ")}`);
  console.error("See https://github.com/PayEz-Net/vibesql-mail-mcp/releases");
  process.exit(1);
}

const dest = path.join(BIN_DIR, platform === "win32" ? "vibesql-mail-mcp.exe" : "vibesql-mail-mcp");
const url = `https://github.com/${REPO}/releases/download/v${VERSION}/${binaryName}`;

if (fs.existsSync(dest)) {
  console.log("vibesql-mail-mcp: binary already exists, skipping download");
  process.exit(0);
}

fs.mkdirSync(BIN_DIR, { recursive: true });

console.log(`vibesql-mail-mcp: downloading ${binaryName} for ${key}...`);

function download(url, dest, redirects) {
  if (redirects > 5) {
    console.error("vibesql-mail-mcp: too many redirects");
    process.exit(1);
  }

  const proto = url.startsWith("https") ? https : require("http");
  proto.get(url, { headers: { "User-Agent": "vibesql-mail-mcp-npm" } }, (res) => {
    if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
      download(res.headers.location, dest, redirects + 1);
      return;
    }

    if (res.statusCode !== 200) {
      console.error(`vibesql-mail-mcp: download failed (HTTP ${res.statusCode})`);
      console.error(`URL: ${url}`);
      console.error("Download manually from: https://github.com/PayEz-Net/vibesql-mail-mcp/releases");
      process.exit(1);
    }

    const file = fs.createWriteStream(dest);
    let downloaded = 0;
    const total = parseInt(res.headers["content-length"], 10) || 0;

    res.on("data", (chunk) => {
      downloaded += chunk.length;
      if (total > 0) {
        const pct = Math.round((downloaded / total) * 100);
        process.stdout.write(`\rvibesql-mail-mcp: ${pct}% (${Math.round(downloaded / 1024 / 1024)}MB)`);
      }
    });

    res.pipe(file);

    file.on("finish", () => {
      file.close();
      console.log("");

      if (platform !== "win32") {
        fs.chmodSync(dest, 0o755);
      }

      console.log(`vibesql-mail-mcp: installed to ${dest}`);
    });
  }).on("error", (err) => {
    console.error(`vibesql-mail-mcp: download error: ${err.message}`);
    console.error("Download manually from: https://github.com/PayEz-Net/vibesql-mail-mcp/releases");
    process.exit(1);
  });
}

download(url, dest, 0);
