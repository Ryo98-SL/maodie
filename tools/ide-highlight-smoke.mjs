#!/usr/bin/env node

import { createHash, randomBytes } from "node:crypto";
import { connect as connectSocket } from "node:net";

const baseUrl = process.argv[2] ?? "http://127.0.0.1:5173/";
const cdpUrl = process.argv[3] ?? "http://127.0.0.1:9226";

const checks = [];

const client = await connectToFirstPage(cdpUrl);
await client.send("Page.enable");
await client.send("Runtime.enable");

try {
  await navigate(baseUrl);
  await waitFor(
    () => typeof window.maodieIdeEditor?.replaceSource === "function",
    "Monaco smoke-test API mounted"
  );
  await waitFor(
    () => document.querySelector("#diagnostics-summary")?.textContent?.includes("0 live"),
    "default source live lexer ready"
  );
  await expectPage("default example highlights tokens", () => ({
    semanticTokens: window.maodieIdeEditor?.getSemanticTokenCount() ?? 0,
    summary: document.querySelector("#diagnostics-summary")?.textContent ?? ""
  }));

  await setEditorSource(`let 名字 = "ok"\n// emoji 😀 beside identifier\n`);
  await waitFor(
    () => document.querySelector("#diagnostics-summary")?.textContent?.includes("0 live"),
    "Chinese and emoji source is lexically clean"
  );
  await setEditorSource(`let 名字前缀 = "ok"\n// emoji 😀 beside identifier\n`);
  await waitFor(
    () => document.querySelector("#diagnostics-summary")?.textContent?.includes("0 live"),
    "Chinese and emoji nearby edit remains clean"
  );
  await expectPage("Chinese identifier and emoji edit keep decorations", () => ({
    semanticTokens: window.maodieIdeEditor?.getSemanticTokenCount() ?? 0,
    summary: document.querySelector("#diagnostics-summary")?.textContent ?? ""
  }));

  await loadSource(`let title = "unterminated\n`);
  await waitFor(
    () => document.querySelector("#diagnostics-summary")?.textContent?.includes("1 live"),
    "unterminated string live diagnostic"
  );
  await expectPage("unterminated string reports live lexer diagnostic", () => ({
    liveDiagnostic: window.maodieIdeEditor?.getLiveMarkerCount() ?? 0,
    semanticTokens: window.maodieIdeEditor?.getSemanticTokenCount() ?? 0,
    summary: document.querySelector("#diagnostics-summary")?.textContent ?? ""
  }));

  await loadSource(`/* open block\nlet x = 1\n`);
  await waitFor(
    () => document.querySelector("#diagnostics-summary")?.textContent?.includes("1 live"),
    "open block comment live diagnostic"
  );
  await loadSource(`/* open block */\nlet x = 1\n`);
  await waitFor(
    () => document.querySelector("#diagnostics-summary")?.textContent?.includes("0 live"),
    "closed block comment clears live diagnostic"
  );
  await expectPage("block comment open and close updates diagnostics", () => ({
    semanticTokens: window.maodieIdeEditor?.getSemanticTokenCount() ?? 0,
    summary: document.querySelector("#diagnostics-summary")?.textContent ?? ""
  }));

  await loadSource(`let x = @\n`);
  await waitFor(
    () => document.querySelector("#diagnostics-summary")?.textContent?.includes("1 live"),
    "illegal character live diagnostic"
  );
  await expectPage("illegal character renders error token and marker", () => ({
    semanticTokens: window.maodieIdeEditor?.getSemanticTokenCount() ?? 0,
    liveDiagnostic: window.maodieIdeEditor?.getLiveMarkerCount() ?? 0,
    text: document.querySelector("#diagnostics-panel")?.textContent ?? ""
  }));

  await navigate(baseUrl);
  await waitFor(
    () => document.querySelector("#diagnostics-summary")?.textContent?.includes("0 live"),
    "default source ready before example switch"
  );
  await clickSelector('[data-example="hello"]');
  await waitFor(
    () => window.maodieIdeEditor?.getSource().includes("Hello world"),
    "Hello World example selected"
  );
  await waitFor(
    () => document.querySelector("#diagnostics-summary")?.textContent?.includes("0 live / not run"),
    "example switch resets compile result"
  );
  await clickSelector("#compile-button");
  await waitFor(
    () => document.body.textContent?.includes("Compiled") && document.body.textContent?.includes("Hello world"),
    "Run compiles current Monaco document"
  );
  await expectPage("example switch and Run compile current document", () => ({
    summary: document.querySelector("#diagnostics-summary")?.textContent ?? "",
    bodyHasCompiled: document.body.textContent?.includes("Compiled") ?? false,
    bodyHasHello: document.body.textContent?.includes("Hello world") ?? false
  }));

  await navigate(baseUrl);
  await waitFor(
    () => document.querySelector("#diagnostics-summary")?.textContent?.includes("0 live"),
    "default source ready before rapid input"
  );
  const perf = await evaluate(`
    (() => {
      const editor = monacoSmokeApi();
      const middle = Array.from({ length: 120 }, (_, i) => "let value" + i + " = " + i).join("\\n") + "\\n";
      replaceMonacoDocument(middle);
      const start = performance.now();
      for (let index = 0; index < 40; index += 1) {
        editor.insertText("x");
      }
      return {
        durationMs: Math.round((performance.now() - start) * 100) / 100,
        length: editor.getSource().length
      };
    })()
  `);
  await waitFor(
    () => document.querySelector("#diagnostics-summary")?.textContent?.includes("0 live / not run"),
    "rapid input settles without stale diagnostic overwrite"
  );
  if (perf.durationMs > 300) {
    throw new Error(`Rapid input dispatch took ${perf.durationMs}ms, expected <= 300ms.`);
  }
  checks.push({
    name: "rapid input performance and stale response settling",
    details: {
      durationMs: perf.durationMs,
      length: perf.length,
      summary: await evaluate(`document.querySelector("#diagnostics-summary")?.textContent ?? ""`)
    }
  });

  console.log(JSON.stringify({ ok: true, baseUrl, checks }, null, 2));
} finally {
  await client.close();
}

async function navigate(url) {
  await client.send("Page.navigate", { url });
  await waitFor(() => document.readyState === "complete", `page load: ${url}`);
}

async function setEditorSource(source) {
  await evaluate(`
    (() => {
      return replaceMonacoDocument(${JSON.stringify(source)});
    })()
  `);
}

async function loadSource(source) {
  const url = new URL(baseUrl);
  url.searchParams.set("source", source);
  await navigate(url.toString());
}

async function clickSelector(selector) {
  await evaluate(`
    (() => {
      const element = document.querySelector(${JSON.stringify(selector)});
      if (!(element instanceof HTMLElement)) {
        throw new Error("Element not found: " + ${JSON.stringify(selector)});
      }
      element.click();
    })()
  `);
}

async function expectPage(name, fn) {
  const details = await evaluate(`(${fn.toString()})()`);
  for (const [key, value] of Object.entries(details)) {
    if (typeof value === "number" && value <= 0) {
      throw new Error(`${name} failed: ${key} was ${value}.`);
    }
    if (typeof value === "boolean" && !value) {
      throw new Error(`${name} failed: ${key} was false.`);
    }
    if (key === "summary" && !String(value).includes("live")) {
      throw new Error(`${name} failed: summary was ${value}.`);
    }
  }
  checks.push({ name, details });
}

async function waitFor(predicate, label, timeoutMs = 10000) {
  const started = Date.now();
  let lastError;
  while (Date.now() - started < timeoutMs) {
    try {
      if (await evaluate(`(${predicate.toString()})()`)) {
        return;
      }
    } catch (error) {
      lastError = error;
    }
    await new Promise((resolve) => setTimeout(resolve, 100));
  }

  let snapshot = "";
  try {
    snapshot = await evaluate(`
      ({
        summary: document.querySelector("#diagnostics-summary")?.textContent ?? "",
        diagnostics: document.querySelector("#diagnostics-panel")?.textContent ?? "",
        editor: window.maodieIdeEditor?.getSource() ?? ""
      })
    `).then((value) => ` snapshot=${JSON.stringify(value)}`);
  } catch {
    snapshot = "";
  }

  throw new Error(
    `Timed out waiting for ${label}${lastError ? `: ${lastError.message}` : ""}${snapshot}`
  );
}

async function evaluate(expression) {
  const response = await client.send("Runtime.evaluate", {
    expression: `
      (() => {
        function monacoSmokeApi() {
          if (!window.maodieIdeEditor) {
            throw new Error("Monaco editor test API was not found.");
          }
          return window.maodieIdeEditor;
        }
        function replaceMonacoDocument(source) {
          return monacoSmokeApi().replaceSource(source);
        }
        return (${expression});
      })()
    `,
    awaitPromise: true,
    returnByValue: true
  });
  if (response.exceptionDetails) {
    throw new Error(
      response.exceptionDetails.exception?.description ??
        response.exceptionDetails.exception?.value ??
        response.exceptionDetails.text
    );
  }
  return response.result.value;
}

async function connectToFirstPage(remoteUrl) {
  const targets = await fetch(`${remoteUrl.replace(/\/$/, "")}/json/list`).then((response) =>
    response.json()
  );
  const target = targets.find((candidate) => candidate.type === "page");
  if (!target?.webSocketDebuggerUrl) {
    throw new Error(`No page target found at ${remoteUrl}.`);
  }
  return connectWebSocket(target.webSocketDebuggerUrl);
}

async function connectWebSocket(url) {
  const socket = await openWebSocketSocket(url);
  const pending = new Map();
  let nextId = 0;

  let buffer = Buffer.alloc(0);
  socket.on("data", (chunk) => {
    buffer = Buffer.concat([buffer, chunk]);
    while (buffer.length > 0) {
      const frame = readFrame(buffer);
      if (!frame) {
        return;
      }
      buffer = buffer.subarray(frame.bytesRead);
      if (frame.opcode === 8) {
        socket.end();
        return;
      }
      if (frame.opcode !== 1) {
        continue;
      }

      const message = JSON.parse(frame.payload.toString("utf8"));
      if (!message.id) {
        continue;
      }
      const request = pending.get(message.id);
      if (!request) {
        continue;
      }
      pending.delete(message.id);
      if (message.error) {
        request.reject(new Error(message.error.message));
        continue;
      }
      request.resolve(message.result ?? {});
    }
  });

  return {
    send(method, params = {}) {
      nextId += 1;
      const id = nextId;
      const promise = new Promise((resolve, reject) => {
        pending.set(id, { resolve, reject });
      });
      socket.write(writeFrame(JSON.stringify({ id, method, params })));
      return promise;
    },
    close() {
      socket.end();
    }
  };
}

async function openWebSocketSocket(url) {
  const parsed = new URL(url);
  const port = Number(parsed.port || 80);
  const socket = connectSocket({ host: parsed.hostname, port });
  const key = randomBytes(16).toString("base64");
  const expectedAccept = createHash("sha1")
    .update(`${key}258EAFA5-E914-47DA-95CA-C5AB0DC85B11`)
    .digest("base64");

  await new Promise((resolve, reject) => {
    socket.once("connect", resolve);
    socket.once("error", reject);
  });

  socket.write(
    [
      `GET ${parsed.pathname}${parsed.search} HTTP/1.1`,
      `Host: ${parsed.host}`,
      "Upgrade: websocket",
      "Connection: Upgrade",
      `Sec-WebSocket-Key: ${key}`,
      "Sec-WebSocket-Version: 13",
      "",
      ""
    ].join("\r\n")
  );

  let handshake = Buffer.alloc(0);
  await new Promise((resolve, reject) => {
    const onData = (chunk) => {
      handshake = Buffer.concat([handshake, chunk]);
      const headerEnd = handshake.indexOf("\r\n\r\n");
      if (headerEnd < 0) {
        return;
      }
      socket.off("data", onData);
      const header = handshake.subarray(0, headerEnd).toString("utf8");
      if (!header.startsWith("HTTP/1.1 101")) {
        reject(new Error(`WebSocket handshake failed: ${header.split("\r\n")[0]}`));
        return;
      }
      if (!header.toLowerCase().includes(`sec-websocket-accept: ${expectedAccept.toLowerCase()}`)) {
        reject(new Error("WebSocket handshake returned an unexpected accept key."));
        return;
      }
      const remainder = handshake.subarray(headerEnd + 4);
      if (remainder.length > 0) {
        socket.unshift(remainder);
      }
      resolve();
    };
    socket.on("data", onData);
    socket.once("error", reject);
  });

  return socket;
}

function readFrame(buffer) {
  if (buffer.length < 2) {
    return undefined;
  }

  const first = buffer[0];
  const second = buffer[1];
  let offset = 2;
  let length = second & 0x7f;
  if (length === 126) {
    if (buffer.length < offset + 2) {
      return undefined;
    }
    length = buffer.readUInt16BE(offset);
    offset += 2;
  } else if (length === 127) {
    if (buffer.length < offset + 8) {
      return undefined;
    }
    length = Number(buffer.readBigUInt64BE(offset));
    offset += 8;
  }

  const masked = (second & 0x80) !== 0;
  let mask;
  if (masked) {
    if (buffer.length < offset + 4) {
      return undefined;
    }
    mask = buffer.subarray(offset, offset + 4);
    offset += 4;
  }

  if (buffer.length < offset + length) {
    return undefined;
  }

  const payload = Buffer.from(buffer.subarray(offset, offset + length));
  if (mask) {
    for (let index = 0; index < payload.length; index += 1) {
      payload[index] ^= mask[index % 4];
    }
  }

  return {
    opcode: first & 0x0f,
    payload,
    bytesRead: offset + length
  };
}

function writeFrame(text) {
  const payload = Buffer.from(text, "utf8");
  const mask = randomBytes(4);
  const headerLength = payload.length < 126 ? 2 : payload.length <= 0xffff ? 4 : 10;
  const frame = Buffer.alloc(headerLength + 4 + payload.length);
  frame[0] = 0x81;
  if (payload.length < 126) {
    frame[1] = 0x80 | payload.length;
  } else if (payload.length <= 0xffff) {
    frame[1] = 0x80 | 126;
    frame.writeUInt16BE(payload.length, 2);
  } else {
    frame[1] = 0x80 | 127;
    frame.writeBigUInt64BE(BigInt(payload.length), 2);
  }
  mask.copy(frame, headerLength);
  for (let index = 0; index < payload.length; index += 1) {
    frame[headerLength + 4 + index] = payload[index] ^ mask[index % 4];
  }
  return frame;
}
