/**
 * SkyCode Runtime Adapter
 *
 * Abstracts whether we're running inside a Tauri window or a plain browser.
 *
 *  - Tauri:  uses `invoke` / `listen` from  @tauri-apps/api
 *  - Web:    uses SSE (EventSource) at http://localhost:4321/api/chat
 */

// ── Detect Tauri ─────────────────────────────────────────────────────────────
export function isTauri(): boolean {
  return typeof (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ !== "undefined";
}

// ── Event callback types ──────────────────────────────────────────────────────
export interface ChunkPayload {
  content: string;
  is_complete: boolean;
}

export type ChunkListener = (payload: ChunkPayload) => void;
export type UnlistenFn    = () => void;

// ── WEB: SSE-based streaming chat ─────────────────────────────────────────────
const WEB_API = (window as unknown as Record<string, unknown>).__SKYCODE_API_URL__ as string
  || "http://localhost:4321";

/**
 * Listen for `message-chunk` events.
 *
 * In Tauri mode wraps `listen()`.
 * In web mode this is a no-op registration that web `sendMessageWeb` drives directly.
 * Returns an unlisten function.
 */
let _webListeners: Set<ChunkListener> = new Set();

export function listenChunks(cb: ChunkListener): UnlistenFn {
  if (isTauri()) {
    // Dynamic import avoids bundling Tauri API in web-only builds
    let unlisten: UnlistenFn = () => {};
    import("@tauri-apps/api/event").then(({ listen }) => {
      listen<ChunkPayload>("message-chunk", (event) => {
        cb(event.payload);
      }).then((fn) => { unlisten = fn; });
    });
    return () => unlisten();
  }

  // Web mode
  _webListeners.add(cb);
  return () => { _webListeners.delete(cb); };
}

// ── Send message ──────────────────────────────────────────────────────────────
export async function sendMessage(
  message: string,
  agent: string,
  model: string,
): Promise<void> {
  if (isTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    await invoke("send_message", { message, agent, model });
    return;
  }

  // Web mode: POST → SSE stream
  const res = await fetch(`${WEB_API}/api/chat`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ message, agent, model }),
  });

  if (!res.ok || !res.body) {
    const errText = await res.text().catch(() => "unknown error");
    _webListeners.forEach((cb) => cb({ content: `Error: ${errText}`, is_complete: false }));
    _webListeners.forEach((cb) => cb({ content: "", is_complete: true }));
    return;
  }

  // Stream the SSE response
  const reader  = res.body.getReader();
  const decoder = new TextDecoder();
  let   buffer  = "";

  try {
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;

      buffer += decoder.decode(value, { stream: true });
      const lines = buffer.split("\n");
      buffer = lines.pop() ?? "";

      for (const line of lines) {
        if (!line.startsWith("data:")) continue;
        const raw = line.slice(5).trim();
        if (!raw) continue;
        try {
          const payload: ChunkPayload = JSON.parse(raw);
          _webListeners.forEach((cb) => cb(payload));
        } catch {/* skip malformed */}
      }
    }
  } finally {
    reader.releaseLock();
  }

  // Ensure is_complete fires even if server didn't send it
  _webListeners.forEach((cb) => cb({ content: "", is_complete: true }));
}
