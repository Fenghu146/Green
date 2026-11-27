#!/usr/bin/env node
// Green's enhanced log plugin: alert, level groups, and a folded code block
// for one overlong line (a compressed log). One JSON request on stdin.

const fs = require("node:fs");
const { StringDecoder } = require("node:string_decoder");

const MAX_LINES = 2000;
const MAX_CELL = 500;
const FOLD_AT = 120;

// A cell's text as Markdown shows it, not as Markdown means it.
function literal(text) {
  return text
    .replace(/\\/g, "\\\\")
    .replace(/[`*_\\[\]<>|]/g, (mark) => `\\${mark}`)
    .replace(/\r\n|\r|\n/g, "<br>");
}

// Above 8 MB the host sends only a path: read the first lines, not the file.
function readHead(path) {
  const fd = fs.openSync(path, "r");
  const buffer = Buffer.allocUnsafe(64 * 1024);
  const decoder = new StringDecoder("utf8");
  const lines = [];
  let pending = "";
  try {
    while (lines.length < MAX_LINES && pending.length < MAX_CELL * 400) {
      const read = fs.readSync(fd, buffer, 0, buffer.length, null);
      if (read === 0) break;
      pending += decoder.write(buffer.subarray(0, read));
      let at;
      while ((at = pending.indexOf("\n")) !== -1 && lines.length < MAX_LINES) {
        lines.push(pending.slice(0, at));
        pending = pending.slice(at + 1);
      }
    }
    if (lines.length < MAX_LINES && pending.trim() !== "") lines.push(pending);
  } finally {
    fs.closeSync(fd);
  }
  return lines;
}

function fold(line) {
  const body = line.slice(0, MAX_CELL * 10);
  const pieces = body.match(new RegExp(`[\\s\\S]{1,${FOLD_AT}}`, "g")) || [];
  const tail = line.length > body.length ? `\n[余下 ${line.length - body.length} 字符未显示]` : "";
  return pieces.join("\n") + tail;
}

function main() {
  const request = JSON.parse(fs.readFileSync(0, "utf8"));
  if (request.apiVersion !== 1) {
    console.error(`this plugin speaks apiVersion 1, not ${request.apiVersion}`);
    return 2;
  }

  let lines;
  let omitted = false;
  if (request.content == null) {
    lines = readHead(request.path);
    omitted = true;
  } else {
    const text = Buffer.from(request.content, "base64")
      .toString("utf8").replace(/^\uFEFF/, "");
    lines = text.split(/\r?\n/).filter((line) => line.trim() !== "").slice(0, MAX_LINES);
  }

  const pattern = /^(\S+)\s+(TRACE|DEBUG|INFO|WARN|WARNING|ERROR|FATAL)\s+(.*)$/i;
  const groups = { error: [], warn: [], other: [] };
  const long = [];
  let errors = 0;
  let warns = 0;
  for (const line of lines) {
    if (!line.trim()) continue;
    const found = line.match(pattern);
    const level = found ? found[2].toUpperCase() : "";
    if (level === "ERROR" || level === "FATAL") errors += 1;
    if (level === "WARN" || level === "WARNING") warns += 1;
    const row = found
      ? [literal(found[1]), literal(level), literal(found[3])]
      : ["—", "—", literal(line)];
    if (row[2].replace(/<br>/g, "").length > MAX_CELL) {
      long.push(fold(line));
    } else if (level === "ERROR" || level === "FATAL") {
      groups.error.push(row);
    } else if (level === "WARN" || level === "WARNING") {
      groups.warn.push(row);
    } else {
      groups.other.push(row);
    }
  }

  const blocks = [];
  if (errors > 0) {
    blocks.push({
      kind: "alert", level: "warning",
      text: `共 ${lines.length} 行，error ${errors} 条，warn ${warns} 条`,
    });
  }
  if (omitted) {
    blocks.push({
      kind: "paragraph",
      text: `文件超过 8 MB，只读取前 ${Math.min(lines.length, MAX_LINES)} 行。`,
    });
  }
  const titles = { error: "错误 / error", warn: "警告 / warn", other: "其他 / other" };
  const shown = [];
  for (const key of ["error", "warn", "other"]) {
    if (groups[key].length === 0) continue;
    if (shown.length) blocks.push({ kind: "rule" });
    blocks.push({ kind: "heading", level: 2, text: titles[key] });
    blocks.push({ kind: "table", head: ["time", "level", "message"], rows: groups[key] });
    shown.push(key);
  }
  if (long.length) {
    if (shown.length) blocks.push({ kind: "rule" });
    blocks.push({ kind: "heading", level: 2, text: "超长行 / folded" });
    blocks.push({ kind: "code", language: "text", text: long.join("\n\n") });
  }
  if (blocks.length === 0) {
    blocks.push({ kind: "paragraph", text: "日志是空的。" });
  }

  process.stdout.write(JSON.stringify({
    apiVersion: 1,
    blocks,
    meta: { lines: lines.length, errors, warns, groups: shown },
  }));
  return 0;
}

process.exitCode = main();
