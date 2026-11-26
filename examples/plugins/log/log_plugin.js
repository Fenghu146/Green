#!/usr/bin/env node
// Green's log plugin: one request on stdin, blocks on stdout.
// The second language on purpose: the contract is JSON, not Rust.

const fs = require("node:fs");

// A cell's text as Markdown shows it, not as Markdown means it.
function literal(text) {
  return text
    .replace(/\\/g, "\\\\")
    .replace(/[`*_\[\]<>|]/g, (mark) => `\\${mark}`)
    .replace(/\r\n|\r|\n/g, "<br>");
}

function main() {
  const request = JSON.parse(fs.readFileSync(0, "utf8"));
  if (request.apiVersion !== 1) {
    console.error(`this plugin speaks apiVersion 1, not ${request.apiVersion}`);
    return 2;
  }

  // Over 8 MB the host sends the path and no bytes; a missing field means the same.
  const content = request.content ?? null;
  const bytes = content === null
    ? fs.readFileSync(request.path)
    : Buffer.from(content, "base64");
  const text = bytes.toString("utf8").replace(/^\uFEFF/, "");

  const lines = text.split(/\r?\n/).filter((line) => line.trim() !== "");
  const pattern = /^(\S+)\s+(TRACE|DEBUG|INFO|WARN|WARNING|ERROR|FATAL)\s+(.*)$/i;
  const rows = [];
  const other = [];
  for (const line of lines) {
    const found = line.match(pattern);
    if (found) {
      rows.push([literal(found[1]), literal(found[2].toUpperCase()), literal(found[3])]);
    } else {
      other.push(line);
    }
  }

  const errors = rows.filter((row) => row[1] === "ERROR" || row[1] === "FATAL").length;
  const blocks = [];
  if (errors > 0) {
    blocks.push({
      kind: "alert",
      level: "warning",
      text: `${errors} of ${lines.length} lines ${errors === 1 ? "is" : "are"} an error`,
    });
  }
  if (rows.length > 0) {
    blocks.push({ kind: "table", head: ["time", "level", "message"], rows });
  }
  if (other.length > 0) {
    blocks.push({ kind: "code", language: "text", text: other.join("\n") });
  }
  // Nothing looked like a log: show it as it is rather than as an empty view.
  if (rows.length === 0 && other.length === 0) {
    blocks.push({ kind: "code", language: "text", text });
  }

  process.stdout.write(JSON.stringify({
    apiVersion: 1,
    blocks,
    meta: { lines: lines.length, errors, matched: rows.length },
  }));
  return 0;
}

process.exitCode = main();
