import { useEffect, useMemo, useRef, useState } from "react";
import ReactMarkdown from "react-markdown";
import {
  Archive,
  ArrowUp,
  ChevronLeft,
  ChevronRight,
  LogOut,
  MessageSquare,
  Moon,
  Pencil,
  Plus,
  Settings2,
  Square,
  Sun,
  Trash2,
  Undo2,
  User,
  X,
} from "lucide-react";
import { cn } from "./lib/utils";
import { isTauri, listenChunks, sendMessage as runtimeSend } from "./lib/runtime";
import { DialogProvider, useDialogs } from "./components/dialogs";

type Role = "user" | "assistant";
type WindowMode = "agent" | "chat";

interface Message { id: string; role: Role; content: string; agent?: string }
interface Chat { id: string; title: string; messages: Message[]; createdAt: number; archived?: boolean }
interface AppSettings {
  model: string;
  customModels: string[];
  ttsEnabled: boolean;
  baseUrl: string;
  apiKey: string;
  permissionMode: "read-only" | "workspace-write" | "danger-full-access";
  approvalMode: "auto" | "ask";
  quickTestMode: boolean;
}
interface LocalUser { id: string; username: string; password: string }
interface UserSession { userId: string; username: string; isGuest: boolean }
interface WorkWindow { id: string; mode: WindowMode; agent: string; chatId: string; order: number }
interface V2Chunk { request_id: string; content: string; is_complete: boolean }
interface RequestRuntime {
  startedAt: number;
  lastChunkAt: number;
  chunkCount: number;
}
interface BackendRequestStatus {
  known: boolean;
  pid: number | null;
  process_alive: boolean;
  bridge_alive: boolean;
  ollama_alive: boolean;
  checkedAt: number;
}

const playAlertTone = () => {
  try {
    const ctx = new (window.AudioContext || (window as typeof window & { webkitAudioContext?: typeof AudioContext }).webkitAudioContext)();
    const osc = ctx.createOscillator();
    const gain = ctx.createGain();
    osc.type = "sine";
    osc.frequency.value = 784;
    gain.gain.setValueAtTime(0.0001, ctx.currentTime);
    gain.gain.exponentialRampToValueAtTime(0.08, ctx.currentTime + 0.02);
    gain.gain.exponentialRampToValueAtTime(0.0001, ctx.currentTime + 0.28);
    osc.connect(gain);
    gain.connect(ctx.destination);
    osc.start();
    osc.stop(ctx.currentTime + 0.3);
  } catch {
    // Ignore audio errors if browser policy blocks autoplay.
  }
};

const requestNotificationPermission = async () => {
  if (typeof Notification === "undefined") return;
  if (Notification.permission === "default") {
    try { await Notification.requestPermission(); } catch { /* ignore */ }
  }
};

const showDesktopNotification = (title: string, body: string) => {
  if (typeof Notification === "undefined") return;
  if (Notification.permission !== "granted") return;
  try {
    new Notification(title, { body, silent: true });
  } catch {
    // Ignore if platform blocks notifications.
  }
};

const AGENTS = ["Central", "Manager", "SQL", "Backend", "Frontend"];
const FALLBACK_MODELS = ["llama3.2:1b", "llama3.1:8b", "dolphin-mistral:7b"];
const MODEL_PACK_PRESETS = [
  {
    id: "coding-pro",
    name: "Coding Pro",
    models: ["qwen3:14b", "qwen2.5-coder:14b", "codestral"],
  },
  {
    id: "reasoning-heavy",
    name: "Reasoning Heavy",
    models: ["deepseek-r1:32b", "deepseek-coder-v2:16b"],
  },
  {
    id: "balanced",
    name: "Balanced",
    models: ["llama4:scout", "qwen3:14b", "codestral"],
  },
];

const CHATS_KEY_BASE = "skycode_chats_v6";
const SETTINGS_KEY_BASE = "skycode_settings_v6";
const USERS_KEY = "skycode_users_v1";
const SESSION_KEY = "skycode_session_v1";
const AUTH_MODE_KEY = "skycode_auth_mode_v1";

const genId = () => Math.random().toString(36).slice(2, 10);
const scopedKey = (base: string, userId: string) => `${base}_${userId}`;

const loadUsers = (): LocalUser[] => {
  try { return JSON.parse(localStorage.getItem(USERS_KEY) ?? "[]"); }
  catch { return []; }
};
const saveUsers = (users: LocalUser[]) => localStorage.setItem(USERS_KEY, JSON.stringify(users));
const loadSession = (): UserSession | null => {
  try { return JSON.parse(localStorage.getItem(SESSION_KEY) ?? "null"); }
  catch { return null; }
};
const saveSession = (s: UserSession | null) => {
  if (s) localStorage.setItem(SESSION_KEY, JSON.stringify(s));
  else localStorage.removeItem(SESSION_KEY);
};
const loadChats = (uid: string): Chat[] => {
  try {
    const raw = JSON.parse(localStorage.getItem(scopedKey(CHATS_KEY_BASE, uid)) ?? "[]") as Chat[];
    return raw.map(c => ({ ...c, archived: c.archived ?? false }));
  }
  catch { return []; }
};
const saveChats = (uid: string, chats: Chat[]) => localStorage.setItem(scopedKey(CHATS_KEY_BASE, uid), JSON.stringify(chats.slice(0, 100)));
const loadSettings = (uid: string): AppSettings => {
  const d: AppSettings = {
    model: "llama3.2:1b",
    customModels: [],
    ttsEnabled: false,
    baseUrl: "http://localhost:4000",
    apiKey: "",
    permissionMode: "workspace-write",
    approvalMode: "ask",
    quickTestMode: true,
  };
  try { return { ...d, ...JSON.parse(localStorage.getItem(scopedKey(SETTINGS_KEY_BASE, uid)) ?? "{}") }; }
  catch { return d; }
};
const saveSettings = (uid: string, s: AppSettings) => localStorage.setItem(scopedKey(SETTINGS_KEY_BASE, uid), JSON.stringify(s));
const makeChat = (): Chat => ({ id: `c_${genId()}`, title: "New conversation", messages: [], createdAt: Date.now(), archived: false });

function SkyLogo({ size = 20 }: { size?: number }) {
  const gid = `lg-${size}`;
  return (
    <svg width={size} height={size} viewBox="0 0 32 32" fill="none" xmlns="http://www.w3.org/2000/svg">
      <defs>
        <radialGradient id={gid} cx="50%" cy="35%" r="65%">
          <stop offset="0%" stopColor="#7dd3fc" />
          <stop offset="50%" stopColor="#0369a1" />
          <stop offset="100%" stopColor="#082f49" />
        </radialGradient>
      </defs>
      <rect width="32" height="32" rx="6" fill={`url(#${gid})`} />
      <ellipse cx="16" cy="16" rx="10" ry="4" stroke="#bae6fd" strokeWidth="0.8" fill="none" opacity="0.7" />
      <ellipse cx="16" cy="16" rx="10" ry="4" stroke="#bae6fd" strokeWidth="0.8" fill="none" opacity="0.7" transform="rotate(60 16 16)" />
      <ellipse cx="16" cy="16" rx="10" ry="4" stroke="#bae6fd" strokeWidth="0.8" fill="none" opacity="0.7" transform="rotate(120 16 16)" />
      <circle cx="16" cy="16" r="3" fill="#fff" />
    </svg>
  );
}

function AuthOverlayWrapper({ onAuth }: { onAuth: (s: UserSession) => void }) {
  return <AuthOverlay onAuth={onAuth} />;
}

function AuthOverlay({ onAuth }: { onAuth: (s: UserSession) => void }) {
  const [mode, setMode] = useState<"login" | "register" | "guest">(() => {
    const m = localStorage.getItem(AUTH_MODE_KEY);
    return m === "login" || m === "register" || m === "guest" ? m : "guest";
  });
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState("");

  const switchMode = (m: "login" | "register" | "guest") => {
    localStorage.setItem(AUTH_MODE_KEY, m);
    setMode(m);
    setError("");
  };

  const submit = () => {
    const u = username.trim();
    if (!u) return setError("Name required");

    if (mode === "guest") {
      const s: UserSession = { userId: `guest_${u.toLowerCase().replace(/\s+/g, "_")}`, username: u, isGuest: true };
      saveSession(s);
      onAuth(s);
      return;
    }

    if (!password.trim()) return setError("Password required");
    const users = loadUsers();

    if (mode === "register") {
      if (users.some(x => x.username.toLowerCase() === u.toLowerCase())) return setError("Username exists");
      const user: LocalUser = { id: `usr_${genId()}`, username: u, password };
      saveUsers([...users, user]);
      const s: UserSession = { userId: user.id, username: user.username, isGuest: false };
      saveSession(s);
      onAuth(s);
      return;
    }

    const found = users.find(x => x.username.toLowerCase() === u.toLowerCase() && x.password === password);
    if (!found) return setError("Invalid credentials");
    const s: UserSession = { userId: found.id, username: found.username, isGuest: false };
    saveSession(s);
    onAuth(s);
  };

  return (
    <div className="auth-overlay">
      <div className="auth-card">
        <div className="auth-head"><SkyLogo size={28} /><h2>SkyCode</h2></div>
        <p className="auth-sub">Login, register or guest profile.</p>
        <div className="auth-tabs">
          <button className={cn("auth-tab", mode === "login" && "auth-tab--active")} onClick={() => switchMode("login")}>Login</button>
          <button className={cn("auth-tab", mode === "register" && "auth-tab--active")} onClick={() => switchMode("register")}>Register</button>
          <button className={cn("auth-tab", mode === "guest" && "auth-tab--active")} onClick={() => switchMode("guest")}>Guest</button>
        </div>
        <div className="auth-form">
          <input className="auth-input" value={username} onChange={e => setUsername(e.target.value)} placeholder={mode === "guest" ? "Guest name" : "Username"} />
          {mode !== "guest" && <input className="auth-input" type="password" value={password} onChange={e => setPassword(e.target.value)} placeholder="Password" />}
          {error && <p className="auth-error">{error}</p>}
          <button className="auth-submit" onClick={submit}>Continue</button>
        </div>
      </div>
    </div>
  );
}

function AppContent() {
  const [session, setSession] = useState<UserSession | null>(() => loadSession());
  const [isDark, setIsDark] = useState(true);
  const [sidebarOpen, setSidebarOpen] = useState(true);
  const [showSettings, setShowSettings] = useState(false);

  const [settings, setSettings] = useState<AppSettings>({
    model: "llama3.2:1b",
    customModels: [],
    ttsEnabled: false,
    baseUrl: "http://localhost:4000",
    apiKey: "",
    permissionMode: "workspace-write",
    approvalMode: "ask",
    quickTestMode: true,
  });
  const [chats, setChats] = useState<Chat[]>([]);
  const [windows, setWindows] = useState<WorkWindow[]>([]);
  const [drafts, setDrafts] = useState<Record<string, string>>({});

  const [ollamaModels, setOllamaModels] = useState<string[]>([]);
  const [ollamaOk, setOllamaOk] = useState<boolean | null>(null);
  const [skyBinOk, setSkyBinOk] = useState<boolean | null>(null);
  const [bridgeOk, setBridgeOk] = useState<boolean | null>(null);
  const [diagDismissed, setDiagDismissed] = useState(false);
  const [modelOpBusy, setModelOpBusy] = useState<string | null>(null);
  const [modelOpMessage, setModelOpMessage] = useState<string>("");
  const [aliasName, setAliasName] = useState("angel0.1");
  const [aliasSource, setAliasSource] = useState("qwen3:14b");
  const [selectedProfile, setSelectedProfile] = useState<string>("");

  const [activeReqByWindow, setActiveReqByWindow] = useState<Record<string, string>>({});
  const [activeMetaByReq, setActiveMetaByReq] = useState<Record<string, { windowId: string; chatId: string; agent: string }>>({});
  const [requestRuntimeByReq, setRequestRuntimeByReq] = useState<Record<string, RequestRuntime>>({});
  const [backendStatusByReq, setBackendStatusByReq] = useState<Record<string, BackendRequestStatus>>({});
  const [queueByWindow, setQueueByWindow] = useState<Record<string, string[]>>({});
  const [activeWindowId, setActiveWindowId] = useState<string | null>(null);
  const [attentionByWindow, setAttentionByWindow] = useState<Record<string, number>>({});
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const [, setChatAbortControllers] = useState<Record<string, AbortController>>({});
  const [showArchivedChats, setShowArchivedChats] = useState(false);
  const [uiClock, setUiClock] = useState<number>(Date.now());

  const [draggingWindowId, setDraggingWindowId] = useState<string | null>(null);
  const [colSplit, setColSplit] = useState(50);
  const [rowSplit, setRowSplit] = useState(50);
  const [dragSplit, setDragSplit] = useState<"col" | "row" | null>(null);
  const workspaceRef = useRef<HTMLDivElement>(null);
  const webReqRef = useRef<string | null>(null);
  const activeMetaByReqRef = useRef<Record<string, { windowId: string; chatId: string; agent: string }>>({});
  const activeWindowIdRef = useRef<string | null>(null);

  const allModels = useMemo(() => [...new Set([...FALLBACK_MODELS, ...ollamaModels, ...settings.customModels])], [ollamaModels, settings.customModels]);
  const sortedWindows = useMemo(() => [...windows].sort((a, b) => a.order - b.order), [windows]);
  const totalQueued = useMemo(() => Object.values(queueByWindow).reduce((s, q) => s + q.length, 0), [queueByWindow]);
  const activeParallel = useMemo(() => Object.keys(activeReqByWindow).length, [activeReqByWindow]);
  const activeChats = useMemo(() => chats.filter(c => !c.archived), [chats]);
  const archivedChats = useMemo(() => chats.filter(c => c.archived), [chats]);
  const activeWindow = useMemo(() => windows.find(w => w.id === activeWindowId) ?? sortedWindows[0], [windows, sortedWindows, activeWindowId]);
  const runtimeSummary = useMemo(() => {
    const reqIds = Object.values(activeReqByWindow);
    if (!reqIds.length) return "Idle";

    const runtimes = reqIds
      .map(id => requestRuntimeByReq[id])
      .filter((v): v is RequestRuntime => Boolean(v));
    if (!runtimes.length) return `Running ${reqIds.length} request(s)...`;

    const now = uiClock;
    const elapsed = runtimes.map(r => Math.floor((now - r.startedAt) / 1000));
    const silent = runtimes.map(r => Math.floor((now - r.lastChunkAt) / 1000));
    const noTokenLongest = runtimes
      .filter(r => r.chunkCount === 0)
      .map(r => Math.floor((now - r.startedAt) / 1000));

    const longestElapsed = Math.max(...elapsed);
    const longestSilent = Math.max(...silent);

    if (noTokenLongest.length) {
      const worstNoToken = Math.max(...noTokenLongest);
      if (worstNoToken >= 120) {
        return `Stalled: no tokens for ${worstNoToken}s (press Stop and retry).`;
      }
      return `Warm-up: no tokens yet for ${worstNoToken}s.`;
    }

    if (longestSilent >= 45) {
      return `Slow stream: last token ${longestSilent}s ago (running ${longestElapsed}s).`;
    }

    return `Streaming... ${longestElapsed}s`;
  }, [activeReqByWindow, requestRuntimeByReq, uiClock]);
  const backendSummary = useMemo(() => {
    if (activeParallel === 0) return "Backend: idle";
    const statuses = Object.values(backendStatusByReq);
    if (!statuses.length) return "Backend: probing worker health...";

    if (statuses.some(s => !s.bridge_alive)) return "Backend issue: SkyBridge offline.";
    if (statuses.some(s => !s.ollama_alive)) return "Backend issue: Ollama offline.";
    if (statuses.some(s => s.known && !s.process_alive)) return "Backend issue: worker exited unexpectedly.";

    const alive = statuses.filter(s => s.process_alive).length;
    const withPid = statuses.filter(s => s.pid != null);
    const pidLabel = withPid.length ? ` (pid ${withPid[0].pid})` : "";
    return `Backend: ${alive}/${statuses.length} worker(s) alive${pidLabel}`;
  }, [activeParallel, backendStatusByReq]);

  const dialogs = useDialogs();

  useEffect(() => { document.documentElement.classList.toggle("dark", isDark); }, [isDark]);

  useEffect(() => {
    if (!session) return;
    const loaded = loadChats(session.userId);
    const seed = loaded.length ? loaded : [makeChat()];
    const firstWindowId = `w_${genId()}`;
    setChats(seed);
    setSettings(loadSettings(session.userId));
    setWindows([{ id: firstWindowId, mode: "agent", agent: "Central", chatId: seed[0].id, order: 0 }]);
    setActiveWindowId(firstWindowId);
    requestNotificationPermission();
  }, [session]);

  useEffect(() => { if (session) saveChats(session.userId, chats); }, [session, chats]);
  useEffect(() => { if (session) saveSettings(session.userId, settings); }, [session, settings]);
  useEffect(() => { activeMetaByReqRef.current = activeMetaByReq; }, [activeMetaByReq]);
  useEffect(() => { activeWindowIdRef.current = activeWindowId; }, [activeWindowId]);
  useEffect(() => {
    const timer = window.setInterval(() => setUiClock(Date.now()), 1000);
    return () => window.clearInterval(timer);
  }, []);

  useEffect(() => {
    if (!isTauri()) return;
    const reqIds = Object.values(activeReqByWindow);
    if (!reqIds.length) {
      setBackendStatusByReq({});
      return;
    }

    let cancelled = false;

    const poll = async () => {
      const { invoke } = await import("@tauri-apps/api/core");
      const now = Date.now();
      const pairs = await Promise.all(reqIds.map(async reqId => {
        try {
          const status = await invoke<Omit<BackendRequestStatus, "checkedAt">>("get_request_status", { requestId: reqId });
          return [reqId, { ...status, checkedAt: now }] as const;
        } catch {
          return [reqId, {
            known: false,
            pid: null,
            process_alive: false,
            bridge_alive: false,
            ollama_alive: false,
            checkedAt: now,
          }] as const;
        }
      }));

      if (cancelled) return;
      setBackendStatusByReq(Object.fromEntries(pairs));
    };

    poll().catch(() => {});
    const timer = window.setInterval(() => {
      poll().catch(() => {});
    }, 2500);

    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, [activeReqByWindow]);

  const refreshModelState = async () => {
    if (isTauri()) {
      const { invoke } = await import("@tauri-apps/api/core");
      // Full diagnostics — single call covers all health checks
      const diag = await invoke<{
        sky_binary_exists: boolean;
        bridge_alive: boolean;
        ollama_alive: boolean;
        sky_binary_path: string;
        exe_dir: string;
        nearby_files: string[];
      }>("get_diagnostics").catch(() => null);
      if (diag) {
        setSkyBinOk(diag.sky_binary_exists);
        setBridgeOk(diag.bridge_alive);
        setOllamaOk(diag.ollama_alive);
      } else {
        // Fallback to legacy get_status
        await invoke<{ ollama: boolean }>("get_status")
          .then(s => setOllamaOk(s.ollama))
          .catch(() => setOllamaOk(false));
      }
      await invoke<string[]>("list_models")
        .then(ms => setOllamaModels(ms))
        .catch(() => {});
      return;
    }
    await fetch("http://localhost:11434/api/tags")
      .then(r => r.json())
      .then(d => {
        setOllamaOk(true);
        setOllamaModels((d.models ?? []).map((m: { name: string }) => m.name));
      })
      .catch(() => setOllamaOk(false));
  };

  useEffect(() => {
    refreshModelState().catch(() => setOllamaOk(false));
  }, []);

  const runModelAction = async (
    actionKey: string,
    command: string,
    payload: Record<string, unknown>,
  ) => {
    if (!isTauri()) {
      setModelOpMessage("Model actions require Tauri desktop mode.");
      return;
    }

    setModelOpBusy(actionKey);
    setModelOpMessage("");

    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const profilePayload = selectedProfile.trim() ? { profile: selectedProfile.trim() } : { profile: null };
      const result = await invoke<{ ok: boolean; stdout?: string }>(command, { ...payload, ...profilePayload });
      const output = (result?.stdout ?? "").trim();
      setModelOpMessage(output || `${command} completed successfully.`);
      await refreshModelState();
    } catch (error) {
      setModelOpMessage(`Action failed: ${String(error)}`);
    } finally {
      setModelOpBusy(null);
    }
  };

  const appendAssistantChunk = (chatId: string, agent: string, chunk: string) => {
    setChats(prev => prev.map(c => {
      if (c.id !== chatId) return c;
      const last = c.messages[c.messages.length - 1];
      if (last?.role === "assistant") {
        return { ...c, messages: [...c.messages.slice(0, -1), { ...last, content: last.content + chunk }] };
      }
      return { ...c, messages: [...c.messages, { id: `m_${genId()}`, role: "assistant", content: chunk, agent }] };
    }));
  };

  const processQueueForWindow = (windowId: string) => {
    setQueueByWindow(prev => {
      const q = prev[windowId] ?? [];
      if (!q.length) return prev;
      const [next, ...rest] = q;
      setTimeout(() => sendFromWindow(windowId, next), 50);
      return { ...prev, [windowId]: rest };
    });
  };

  const clearAttention = (windowId: string) => {
    setAttentionByWindow(prev => {
      if (!prev[windowId]) return prev;
      const next = { ...prev };
      delete next[windowId];
      return next;
    });
  };

  const notifyCompletion = (windowId: string, agent: string) => {
    const shouldNotify = document.hidden || activeWindowIdRef.current !== windowId;
    if (!shouldNotify) return;
    setAttentionByWindow(prev => ({ ...prev, [windowId]: (prev[windowId] ?? 0) + 1 }));
    playAlertTone();
    showDesktopNotification("SkyCode", `${agent} finished and is waiting for your next input.`);
  };

  const confirmAction = (message: string, title = "Confirm"): Promise<boolean> => {
    return dialogs.showConfirm({ title, message, confirmText: "Confirm", cancelText: "Cancel" });
  };

  useEffect(() => {
    if (isTauri()) {
      let stop = () => {};
      import("@tauri-apps/api/event").then(({ listen }) => {
        listen<V2Chunk>("message-chunk-v2", event => {
          const p = event.payload;
          const meta = activeMetaByReqRef.current[p.request_id];
          if (!meta) return;

          if (p.is_complete) {
            notifyCompletion(meta.windowId, meta.agent);
            setActiveReqByWindow(prev => {
              const n = { ...prev };
              delete n[meta.windowId];
              return n;
            });
            setActiveMetaByReq(prev => {
              const n = { ...prev };
              delete n[p.request_id];
              return n;
            });
            setRequestRuntimeByReq(prev => {
              const n = { ...prev };
              delete n[p.request_id];
              return n;
            });
            processQueueForWindow(meta.windowId);
            return;
          }

          if (!p.content) return;
          setRequestRuntimeByReq(prev => {
            const current = prev[p.request_id];
            if (!current) return prev;
            return {
              ...prev,
              [p.request_id]: {
                ...current,
                lastChunkAt: Date.now(),
                chunkCount: current.chunkCount + 1,
              },
            };
          });
          appendAssistantChunk(meta.chatId, meta.agent, p.content);
        }).then(un => { stop = un; });
      });
      return () => stop();
    }

    const un = listenChunks(payload => {
      const reqId = webReqRef.current;
      if (!reqId) return;
      const meta = activeMetaByReqRef.current[reqId];
      if (!meta) return;

      if (payload.is_complete) {
        notifyCompletion(meta.windowId, meta.agent);
        setActiveReqByWindow(prev => {
          const n = { ...prev };
          delete n[meta.windowId];
          return n;
        });
        setActiveMetaByReq(prev => {
          const n = { ...prev };
          delete n[reqId];
          return n;
        });
        setRequestRuntimeByReq(prev => {
          const n = { ...prev };
          delete n[reqId];
          return n;
        });
        webReqRef.current = null;
        processQueueForWindow(meta.windowId);
        return;
      }

      if (!payload.content) return;
      setRequestRuntimeByReq(prev => {
        const current = prev[reqId];
        if (!current) return prev;
        return {
          ...prev,
          [reqId]: {
            ...current,
            lastChunkAt: Date.now(),
            chunkCount: current.chunkCount + 1,
          },
        };
      });
      appendAssistantChunk(meta.chatId, meta.agent, payload.content);
    });
    return () => un();
  }, []);

  useEffect(() => {
    if (!dragSplit) return;
    const move = (e: MouseEvent) => {
      const el = workspaceRef.current;
      if (!el) return;
      const r = el.getBoundingClientRect();
      if (dragSplit === "col") setColSplit(Math.max(20, Math.min(80, ((e.clientX - r.left) / r.width) * 100)));
      if (dragSplit === "row") setRowSplit(Math.max(20, Math.min(80, ((e.clientY - r.top) / r.height) * 100)));
    };
    const up = () => setDragSplit(null);
    window.addEventListener("mousemove", move);
    window.addEventListener("mouseup", up);
    return () => { window.removeEventListener("mousemove", move); window.removeEventListener("mouseup", up); };
  }, [dragSplit]);

  const sendFromWindow = async (windowId: string, forcedText?: string) => {
    clearAttention(windowId);
    setActiveWindowId(windowId);
    const w = windows.find(x => x.id === windowId);
    if (!w) return;
    const text = (forcedText ?? drafts[windowId] ?? "").trim();
    if (!text) return;

    // Architecture rule:
    // - chat mode: no permission prompt
    // - agent mode: always require explicit confirmation
    if (w.mode === "agent") {
      const approvalMessage = `Send this prompt with ${settings.permissionMode} permissions?\n\n${text.slice(0, 260)}${text.length > 260 ? "..." : ""}`;
      const approved = await confirmAction(approvalMessage, "Run Request");
      if (!approved) return;
    }

    const finalText = settings.quickTestMode
      ? `${text}\n\n[TESTING MODE] Reply quickly in 1-3 short sentences. Avoid tool usage unless absolutely required.`
      : text;

    if (activeReqByWindow[windowId]) {
      setQueueByWindow(prev => ({ ...prev, [windowId]: [...(prev[windowId] ?? []), text] }));
      setDrafts(prev => ({ ...prev, [windowId]: "" }));
      return;
    }

    let chat = chats.find(c => c.id === w.chatId);
    if (!chat) {
      chat = makeChat();
      setChats(prev => [chat!, ...prev]);
      setWindows(prev => prev.map(x => x.id === w.id ? { ...x, chatId: chat!.id } : x));
    }

    const agent = w.mode === "chat" ? "Chat" : w.agent;

    setChats(prev => prev.map(c => c.id !== chat!.id ? c : {
      ...c,
      title: c.messages.length === 0 ? text.slice(0, 55) : c.title,
      messages: [...c.messages, { id: `m_${genId()}`, role: "user", content: text, agent }],
    }));

    setDrafts(prev => ({ ...prev, [windowId]: "" }));

    const reqId = `req_${Date.now()}_${genId()}`;
    setActiveReqByWindow(prev => ({ ...prev, [windowId]: reqId }));
    setActiveMetaByReq(prev => ({ ...prev, [reqId]: { windowId, chatId: chat!.id, agent } }));
    setRequestRuntimeByReq(prev => ({
      ...prev,
      [reqId]: {
        startedAt: Date.now(),
        lastChunkAt: Date.now(),
        chunkCount: 0,
      },
    }));

    // Chat mode: bypass sky CLI, talk directly to bridge for near-instant response
    if (w.mode === "chat") {
      const historyForApi = [...chat!.messages, { id: "_", role: "user" as const, content: finalText, agent: "Chat" }];
      sendChatDirectly(windowId, { ...chat!, messages: historyForApi }, settings.model, settings.baseUrl, settings.apiKey, reqId);
      return;
    }

    if (isTauri()) {
      const { invoke } = await import("@tauri-apps/api/core");
      invoke("send_message_v2", {
        message: finalText,
        agent,
        model: settings.model,
        requestId: reqId,
        baseUrl: settings.baseUrl,
        apiKey: settings.apiKey,
        permissionMode: settings.permissionMode,
      }).catch(err => {
        appendAssistantChunk(chat!.id, agent, `Error: ${err}`);
        setActiveReqByWindow(prev => { const n = { ...prev }; delete n[windowId]; return n; });
        setActiveMetaByReq(prev => { const n = { ...prev }; delete n[reqId]; return n; });
        setRequestRuntimeByReq(prev => { const n = { ...prev }; delete n[reqId]; return n; });
      });
    } else {
      webReqRef.current = reqId;
      runtimeSend(text, agent, settings.model).catch(err => appendAssistantChunk(chat!.id, agent, `Error: ${err}`));
    }
  };

  const stopWindow = async (windowId: string) => {
    const reqId = activeReqByWindow[windowId];
    if (!reqId) return;

    if (isTauri()) {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("cancel_message_v2", { requestId: reqId }).catch(() => {});
    }

    setActiveReqByWindow(prev => {
      const meta = activeMetaByReq[reqId];
      // Abort direct bridge fetch for chat mode
      if (meta?.agent === "Chat") {
        setChatAbortControllers(prev2 => {
          prev2[reqId]?.abort();
          const n = { ...prev2 }; delete n[reqId]; return n;
        });
      }
      const n = { ...prev };
      delete n[windowId];
      return n;
    });
    setActiveMetaByReq(prev => {
      const n = { ...prev };
      delete n[reqId];
      return n;
    });
    setRequestRuntimeByReq(prev => {
      const n = { ...prev };
      delete n[reqId];
      return n;
    });
    processQueueForWindow(windowId);
  };

  // ── Chat-mode: send directly to bridge, bypassing sky CLI ─────────────────
  const sendChatDirectly = async (
    windowId: string,
    chat: Chat,
    model: string,
    baseUrl: string,
    apiKey: string,
    reqId: string,
  ): Promise<void> => {
    const bridgeUrl = (baseUrl || "http://localhost:4000").replace(/\/$/, "");
    const controller = new AbortController();
    setChatAbortControllers(prev => ({ ...prev, [reqId]: controller }));

    // Build Anthropic messages from the current chat (including the new user message already added)
    const apiMessages = chat.messages.map(m => ({ role: m.role as "user" | "assistant", content: m.content }));

    const cleanUp = () => {
      setChatAbortControllers(prev => { const n = { ...prev }; delete n[reqId]; return n; });
      setActiveReqByWindow(prev => { const n = { ...prev }; delete n[windowId]; return n; });
      setActiveMetaByReq(prev => { const n = { ...prev }; delete n[reqId]; return n; });
      setRequestRuntimeByReq(prev => { const n = { ...prev }; delete n[reqId]; return n; });
    };

    try {
      const response = await fetch(`${bridgeUrl}/v1/messages`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "x-api-key": apiKey || "ollama",
          "anthropic-version": "2023-06-01",
        },
        body: JSON.stringify({ model, max_tokens: 4096, stream: true, messages: apiMessages }),
        signal: controller.signal,
      });

      if (!response.ok) {
        const errText = await response.text().catch(() => String(response.status));
        appendAssistantChunk(chat.id, "Chat", `Error ${response.status}: ${errText}`);
        notifyCompletion(windowId, "Chat");
        cleanUp();
        processQueueForWindow(windowId);
        return;
      }

      const reader = response.body!.getReader();
      const decoder = new TextDecoder();
      let buffer = "";

      outerLoop: while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        buffer += decoder.decode(value, { stream: true });
        const events = buffer.split("\n\n");
        buffer = events.pop() ?? "";

        for (const event of events) {
          if (!event.trim()) continue;
          let eventType = "";
          let dataStr = "";
          for (const line of event.split("\n")) {
            if (line.startsWith("event: ")) eventType = line.slice(7).trim();
            if (line.startsWith("data: ")) dataStr = line.slice(6).trim();
          }

          if (eventType === "content_block_delta" && dataStr) {
            try {
              const data = JSON.parse(dataStr);
              if (data.delta?.type === "text_delta" && data.delta?.text) {
                setRequestRuntimeByReq(prev => {
                  const cur = prev[reqId];
                  if (!cur) return prev;
                  return { ...prev, [reqId]: { ...cur, lastChunkAt: Date.now(), chunkCount: cur.chunkCount + 1 } };
                });
                appendAssistantChunk(chat.id, "Chat", data.delta.text);
              }
            } catch { /* ignore malformed SSE */ }
          }

          if (eventType === "message_stop") break outerLoop;
        }
      }

      notifyCompletion(windowId, "Chat");
    } catch (err: unknown) {
      if ((err as { name?: string })?.name !== "AbortError") {
        appendAssistantChunk(chat.id, "Chat", `Error: ${String(err)}`);
        notifyCompletion(windowId, "Chat");
      }
    } finally {
      cleanUp();
      processQueueForWindow(windowId);
    }
  };

  const addWindow = () => {
    if (windows.length >= 4) return;
    const chat = makeChat();
    const winId = `w_${genId()}`;
    setChats(prev => [chat, ...prev]);
    setWindows(prev => [...prev, { id: winId, mode: "chat", agent: "Central", chatId: chat.id, order: prev.length }]);
    setActiveWindowId(winId);
  };

  const selectConversation = (chatId: string) => {
    const targetWindowId = activeWindow?.id ?? sortedWindows[0]?.id;
    if (!targetWindowId) return;
    setActiveWindowId(targetWindowId);
    setWindows(prev => prev.map(w => w.id === targetWindowId ? { ...w, chatId } : w));
    clearAttention(targetWindowId);
  };

  const renameConversation = async (chatId: string) => {
    const chat = chats.find(c => c.id === chatId);
    if (!chat) return;
    const nextTitle = await dialogs.showPrompt({
      title: "Rename Conversation",
      message: "Enter a new name for this conversation:",
      defaultValue: chat.title,
    });
    if (!nextTitle?.trim()) return;
    setChats(prev => prev.map(c => c.id === chatId ? { ...c, title: nextTitle.trim() } : c));
  };

  const archiveConversation = (chatId: string) => {
    setChats(prev => {
      const next = prev.map(c => c.id === chatId ? { ...c, archived: true } : c);
      let fallback = next.find(c => !c.archived);
      if (!fallback) {
        fallback = makeChat();
        next.unshift(fallback);
      }
      setWindows(wins => wins.map(w => w.chatId === chatId ? { ...w, chatId: fallback!.id } : w));
      return next;
    });
  };

  const unarchiveConversation = (chatId: string) => {
    setChats(prev => prev.map(c => c.id === chatId ? { ...c, archived: false } : c));
  };

  const deleteAllArchivedConversations = async () => {
    const archivedCount = chats.filter(c => c.archived).length;
    if (!archivedCount) return;

    const deleteMessage = `Delete all ${archivedCount} archived conversation(s)? This action cannot be undone.`;
    const confirmed = await confirmAction(deleteMessage, "Delete All Archived");
    if (!confirmed) return;

    setChats(prev => {
      const remaining = prev.filter(c => !c.archived);
      if (!remaining.length) {
        const fresh = makeChat();
        setWindows(wins => wins.map(w => ({ ...w, chatId: fresh.id })));
        return [fresh];
      }

      const fallback = remaining[0];
      setWindows(wins => wins.map(w => {
        const current = prev.find(c => c.id === w.chatId);
        return current?.archived ? { ...w, chatId: fallback.id } : w;
      }));
      return remaining;
    });
  };

  const deleteConversation = async (chatId: string) => {
    const deleteMessage = "Are you sure you want to delete this conversation permanently? This action cannot be undone.";
    const confirmed = await confirmAction(deleteMessage, "Delete Conversation");
    if (!confirmed) return;
    setChats(prev => {
      const remaining = prev.filter(c => c.id !== chatId);
      if (!remaining.length) {
        const fresh = makeChat();
        setWindows(wins => wins.map(w => ({ ...w, chatId: fresh.id })));
        return [fresh];
      }
      const fallback = remaining.find(c => !c.archived) ?? remaining[0];
      setWindows(wins => wins.map(w => w.chatId === chatId ? { ...w, chatId: fallback.id } : w));
      return remaining;
    });
  };

  const cleanupStuckConversations = async () => {
    const stuckIds = chats
      .filter(c => !c.archived)
      .filter(c => c.messages.length > 0 && c.messages.every(m => m.role === "user"))
      .map(c => c.id);

    if (!stuckIds.length) {
      await dialogs.showAlert({
        title: "No Stuck Conversations",
        message: "All conversations have at least one assistant response or are archived.",
      });
      return;
    }

    const shouldArchive = await dialogs.showConfirm({
      title: "Archive Stuck Conversations",
      message: `Found ${stuckIds.length} conversation(s) with only user messages.\n\nArchive these conversations now?`,
      confirmText: "Archive",
      cancelText: "Keep",
      isDangerous: false,
    });
    if (!shouldArchive) return;

    let nextChats = chats.map(c => stuckIds.includes(c.id) ? { ...c, archived: true } : c);
    let targetChat = nextChats.find(c => !c.archived);
    if (!targetChat) {
      targetChat = makeChat();
      nextChats = [targetChat, ...nextChats];
    }

    setChats(nextChats);
    setWindows(prev => prev.map(w => stuckIds.includes(w.chatId) ? { ...w, chatId: targetChat!.id } : w));
  };

  const moveWindow = (id: string, dir: -1 | 1) => {
    const s = [...windows].sort((a, b) => a.order - b.order);
    const i = s.findIndex(x => x.id === id);
    const j = i + dir;
    if (i < 0 || j < 0 || j >= s.length) return;
    const t = s[i].order;
    s[i].order = s[j].order;
    s[j].order = t;
    setWindows([...s]);
  };

  const removeWindow = (id: string) => {
    if (windows.length <= 1) return;
    setWindows(prev => prev.filter(w => w.id !== id).map((w, i) => ({ ...w, order: i })));
    setAttentionByWindow(prev => {
      if (!prev[id]) return prev;
      const next = { ...prev };
      delete next[id];
      return next;
    });
    setActiveReqByWindow(prev => {
      if (!prev[id]) return prev;
      const next = { ...prev };
      delete next[id];
      return next;
    });
    if (activeWindowId === id) {
      const nextWindow = sortedWindows.find(w => w.id !== id);
      setActiveWindowId(nextWindow?.id ?? null);
    }
  };

  const newChat = () => {
    const c = makeChat();
    setChats(prev => [c, ...prev]);
    setWindows(prev => prev.map(w => w.id === activeWindowId ? { ...w, chatId: c.id } : w));
  };

  const safeUninstall = async () => {
    const dataChoice = await dialogs.showPrompt({
      title: "Uninstall Data Management",
      message: "What should we do with your saved data?\n\nOptions: keep | archive | delete",
      defaultValue: "keep",
      placeholder: "keep",
    });
    if (!dataChoice) return;
    const trimmedChoice = dataChoice.trim().toLowerCase();
    if (!["keep", "archive", "delete"].includes(trimmedChoice)) {
      await dialogs.showAlert({
        title: "Invalid Option",
        message: `"${dataChoice}" is not a valid choice.\n\nValid options are: keep, archive, or delete`,
      });
      return;
    }
    const deleteTemp = await dialogs.showConfirm({
      title: "Delete Temporary Memory",
      message: "Delete temporary memory files created during this session?",
      confirmText: "Delete",
      cancelText: "Keep",
    });
    const clearLocalData = trimmedChoice !== "keep";
    const deleteExternalData = trimmedChoice === "delete" && await dialogs.showConfirm({
      title: "Delete External Data",
      message: "Delete external saved data (e.g. ~/.skycode, ~/.codex)?\n\nThis action cannot be undone.",
      confirmText: "Delete",
      cancelText: "Keep",
      isDangerous: true,
    });

    if (session) {
      const chatsKey = scopedKey(CHATS_KEY_BASE, session.userId);
      const settingsKey = scopedKey(SETTINGS_KEY_BASE, session.userId);
      if (trimmedChoice === "archive") {
        const backupKey = `${chatsKey}_backup_${Date.now()}`;
        localStorage.setItem(backupKey, localStorage.getItem(chatsKey) ?? "[]");
      }
      if (trimmedChoice !== "keep") {
        localStorage.removeItem(chatsKey);
        localStorage.removeItem(settingsKey);
      }
    }
    if (trimmedChoice === "delete") {
      localStorage.removeItem(USERS_KEY);
      localStorage.removeItem(SESSION_KEY);
      localStorage.removeItem(AUTH_MODE_KEY);
    }

    if (isTauri()) {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("safe_uninstall", { deleteTemp, clearLocalData, deleteExternalData }).catch(() => {});
    }
    setShowSettings(false);
  };

  const logout = () => {
    saveSession(null);
    setSession(null);
    setShowSettings(false);
  };

  if (!session) return <AuthOverlayWrapper onAuth={setSession} />;

  const layoutClass = `workspace-grid--${Math.min(sortedWindows.length, 4)}`;
  const gridStyle: React.CSSProperties =
    sortedWindows.length === 1
      ? { gridTemplateColumns: "1fr", gridTemplateRows: "1fr" }
      : sortedWindows.length === 2
        ? { gridTemplateColumns: `${colSplit}% ${100 - colSplit}%`, gridTemplateRows: "1fr" }
        : { gridTemplateColumns: `${colSplit}% ${100 - colSplit}%`, gridTemplateRows: `${rowSplit}% ${100 - rowSplit}%` };

  return (
    <div className={cn("app", isDark ? "theme-dark" : "theme-light", !sidebarOpen && "app--collapsed")}> 
      <aside className="sidebar">
        <div className="sb-header">
          {sidebarOpen ? (
            <>
              <div className="sb-logo"><SkyLogo size={22} /><span>SkyCode</span></div>
              <button className="icon-btn" onClick={() => setSidebarOpen(false)}><ChevronLeft size={15} /></button>
            </>
          ) : (
            <button className="icon-btn sb-expand" onClick={() => setSidebarOpen(true)}><SkyLogo size={20} /></button>
          )}
        </div>
        {sidebarOpen && (
          <>
            <button className="new-chat-btn" onClick={newChat}><Plus size={14} /><span>New chat</span></button>
            <button className="new-chat-btn new-chat-btn--ghost" onClick={cleanupStuckConversations}><X size={14} /><span>Clean stuck</span></button>
            <nav className="chat-list">
              {activeChats.map(chat => (
                <div
                  key={chat.id}
                  className={cn("chat-item", activeWindow?.chatId === chat.id && "chat-item--active")}
                  onClick={() => selectConversation(chat.id)}
                >
                  <MessageSquare size={12} />
                  <span className="chat-item-title">{chat.title}</span>
                  <div className="chat-item-actions">
                    <button className="chat-item-del" title="Rename" onClick={e => { e.stopPropagation(); renameConversation(chat.id); }}><Pencil size={12} /></button>
                    <button className="chat-item-del" title="Archive" onClick={e => { e.stopPropagation(); archiveConversation(chat.id); }}><Archive size={12} /></button>
                    <button className="chat-item-del" title="Delete" onClick={e => { e.stopPropagation(); deleteConversation(chat.id); }}><Trash2 size={12} /></button>
                  </div>
                </div>
              ))}
              {archivedChats.length > 0 && (
                <div className="chat-group">
                  <div className="chat-group-header">
                    <button className="chat-archive-toggle" onClick={() => setShowArchivedChats(v => !v)}>
                      {showArchivedChats ? "Hide archived" : `Show archived (${archivedChats.length})`}
                    </button>
                    {showArchivedChats && (
                      <button className="chat-item-del chat-item-del--always" title="Delete all archived" onClick={deleteAllArchivedConversations}>
                        <Trash2 size={12} />
                      </button>
                    )}
                  </div>
                  {showArchivedChats && archivedChats.map(chat => (
                    <div key={chat.id} className="chat-item chat-item--archived">
                      <Archive size={12} />
                      <span className="chat-item-title">{chat.title}</span>
                      <div className="chat-item-actions">
                        <button className="chat-item-del chat-item-del--always" title="Unarchive" onClick={() => unarchiveConversation(chat.id)}><Undo2 size={12} /></button>
                        <button className="chat-item-del chat-item-del--always" title="Delete" onClick={() => deleteConversation(chat.id)}><Trash2 size={12} /></button>
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </nav>
          </>
        )}
      </aside>

      <main className="main">
        <header className="topbar">
          {!sidebarOpen && <button className="icon-btn" onClick={() => setSidebarOpen(true)}><SkyLogo size={18} /></button>}
          <span className="topbar-model">{settings.model}</span>
          <div className="topbar-actions">
            <span className="user-pill"><User size={12} /> {session.username}</span>
            <button className="icon-btn" onClick={addWindow} title="Add window"><Plus size={14} /></button>
            <button className="icon-btn" onClick={() => setIsDark(d => !d)} title="Theme">{isDark ? <Sun size={14} /> : <Moon size={14} />}</button>
            <button className="icon-btn" onClick={() => setShowSettings(true)} title="Settings"><Settings2 size={14} /></button>
          </div>
        </header>

        {/* Diagnostic banner: shows only when something is not working */}
        {!diagDismissed && (skyBinOk === false || bridgeOk === false || ollamaOk === false) && (
          <div className="diag-banner">
            <div className="diag-banner-items">
              {skyBinOk === false && (
                <span className="diag-item diag-item--error">
                  ❌ Agent binary missing — reinstall SkyCode
                </span>
              )}
              {bridgeOk === false && (
                <span className="diag-item diag-item--error">
                  ❌ SkyBridge offline (port 4000) — restart app
                </span>
              )}
              {ollamaOk === false && (
                <span className="diag-item diag-item--warn">
                  ⚠️ Ollama not running — run: <code>ollama serve</code>
                </span>
              )}
            </div>
            <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
              <button className="diag-retry-btn" onClick={() => { setDiagDismissed(false); refreshModelState(); }} title="Retry">
                ↺ Retry
              </button>
              <button className="icon-btn" onClick={() => setDiagDismissed(true)}><X size={13} /></button>
            </div>
          </div>
        )}

        <div className="workspace-wrap" ref={workspaceRef}>
          <div className={cn("workspace-grid", layoutClass)} style={gridStyle}>
            {sortedWindows.map(w => {
              const chat = chats.find(c => c.id === w.chatId);
              const messages = chat?.messages ?? [];
              const reqId = activeReqByWindow[w.id];
              const reqRuntime = reqId ? requestRuntimeByReq[reqId] : undefined;
              const elapsedSec = reqRuntime ? Math.max(0, Math.floor((uiClock - reqRuntime.startedAt) / 1000)) : 0;
              const silentSec = reqRuntime ? Math.max(0, Math.floor((uiClock - reqRuntime.lastChunkAt) / 1000)) : 0;
              const waitStatus = !reqRuntime
                ? ""
                : reqRuntime.chunkCount === 0 && elapsedSec < 20
                  ? `Starting model... ${elapsedSec}s`
                  : reqRuntime.chunkCount === 0 && elapsedSec < 120
                    ? `No tokens yet (${elapsedSec}s). Model warm-up is in progress.`
                    : reqRuntime.chunkCount === 0
                      ? `Still running after ${elapsedSec}s with no tokens. You can press Stop safely.`
                      : silentSec >= 45
                        ? `Processing... last token ${silentSec}s ago.`
                        : `Receiving output... ${elapsedSec}s`;

              return (
                <section
                  key={w.id}
                  className="workspace-window"
                  draggable
                  onMouseDown={() => {
                    setActiveWindowId(w.id);
                    clearAttention(w.id);
                  }}
                  onDragStart={() => setDraggingWindowId(w.id)}
                  onDragOver={e => e.preventDefault()}
                  onDrop={() => {
                    if (!draggingWindowId || draggingWindowId === w.id) return;
                    const s = [...sortedWindows];
                    const a = s.findIndex(x => x.id === draggingWindowId);
                    const b = s.findIndex(x => x.id === w.id);
                    if (a < 0 || b < 0) return;
                    const t = s[a].order;
                    s[a].order = s[b].order;
                    s[b].order = t;
                    setWindows([...s]);
                    setDraggingWindowId(null);
                  }}
                >
                  <div className="workspace-window-head">
                    <select className="workspace-select" value={w.mode} onChange={e => setWindows(prev => prev.map(x => x.id === w.id ? { ...x, mode: e.target.value as WindowMode } : x))}>
                      <option value="chat">Chat mode</option>
                      <option value="agent">Agent mode</option>
                    </select>
                    {w.mode === "agent" && (
                      <select className="workspace-select" value={w.agent} onChange={e => setWindows(prev => prev.map(x => x.id === w.id ? { ...x, agent: e.target.value } : x))}>
                        {AGENTS.map(a => <option key={a} value={a}>{a}</option>)}
                      </select>
                    )}
                    <span className="workspace-chat-label">{chat?.title ?? "Conversation"}</span>
                    {(attentionByWindow[w.id] ?? 0) > 0 && <span className="workspace-attention">{attentionByWindow[w.id]}</span>}
                    <button className="icon-btn" onClick={() => moveWindow(w.id, -1)}><ChevronLeft size={13} /></button>
                    <button className="icon-btn" onClick={() => moveWindow(w.id, 1)}><ChevronRight size={13} /></button>
                    <button className="icon-btn" onClick={() => removeWindow(w.id)}><X size={13} /></button>
                  </div>

                  <div className="workspace-window-body">
                    {messages.length === 0 ? <div className="workspace-empty">No messages yet</div> : messages.map(m => (
                      <div key={m.id} className={cn("msg", m.role === "user" ? "msg--user" : "msg--ai")}>
                        <p className="msg-label">{m.role === "user" ? "You" : (m.agent ?? "SkyCode")}</p>
                        <div className="msg-body"><ReactMarkdown>{m.content}</ReactMarkdown></div>
                      </div>
                    ))}
                    {reqId && (
                      <div className="msg msg--ai">
                        <p className="msg-label">Thinking</p>
                        <div className="msg-body">
                          <div className="msg-thinking"><span /><span /><span /></div>
                          <p className="msg-thinking-status">{waitStatus}</p>
                        </div>
                      </div>
                    )}
                  </div>

                  <div className="workspace-window-input">
                    <textarea
                      className="input-ta"
                      rows={1}
                      value={drafts[w.id] ?? ""}
                      onChange={e => {
                        clearAttention(w.id);
                        setDrafts(prev => ({ ...prev, [w.id]: e.target.value }));
                      }}
                      onFocus={() => {
                        setActiveWindowId(w.id);
                        clearAttention(w.id);
                      }}
                      onKeyDown={e => {
                        if (e.key === "Enter" && !e.shiftKey) {
                          e.preventDefault();
                          sendFromWindow(w.id);
                        }
                      }}
                      placeholder={w.mode === "chat" ? "Type in simple chat mode..." : `Message ${w.agent}...`}
                    />
                    {reqId
                      ? <button className="send-btn send-btn--stop" onClick={() => stopWindow(w.id)}><Square size={14} fill="currentColor" /></button>
                      : <button className="send-btn" onClick={() => sendFromWindow(w.id)}><ArrowUp size={14} /></button>}
                  </div>
                </section>
              );
            })}
          </div>

          {sortedWindows.length >= 2 && <div className="splitter splitter--col" style={{ left: `${colSplit}%` }} onMouseDown={() => setDragSplit("col")} />}
          {sortedWindows.length >= 3 && <div className="splitter splitter--row" style={{ top: `${rowSplit}%` }} onMouseDown={() => setDragSplit("row")} />}
        </div>

        <p className="input-hint">Windows: {sortedWindows.length}/4 · Queued: {totalQueued} · Parallel active: {activeParallel} · {runtimeSummary} · {backendSummary}</p>
      </main>

      {showSettings && (
        <div className="settings-overlay" onClick={() => setShowSettings(false)}>
          <div className="settings-panel" onClick={e => e.stopPropagation()}>
            <div className="sp-header">
              <div className="sp-logo-row"><SkyLogo size={20} /><span className="sp-title">Settings</span></div>
              <button className="icon-btn" onClick={() => setShowSettings(false)}><X size={14} /></button>
            </div>
            <div className="sp-body">
              <section className="sp-section">
                <h3 className="sp-section-title">Profile</h3>
                <div className="sp-input-row">
                  <label style={{ display: "block", marginBottom: 6, color: "var(--text-dim)", fontSize: 12 }}>Config profile (blank = default)</label>
                  <input
                    className="sp-input"
                    value={selectedProfile}
                    onChange={e => setSelectedProfile(e.target.value)}
                    placeholder="e.g. work, home, prod"
                  />
                </div>
                <p className="sp-hint" style={{ marginTop: 4 }}>Selects ~/{selectedProfile ? `.skycode-${selectedProfile}` : ".skycode"}/config.toml</p>
              </section>
              <section className="sp-section">
                <h3 className="sp-section-title">Permissions</h3>
                <div className="sp-input-row" style={{ marginBottom: 8 }}>
                  <label style={{ display: "block", marginBottom: 6, color: "var(--text-dim)", fontSize: 12 }}>Permission mode</label>
                  <select
                    className="sp-input"
                    value={settings.permissionMode}
                    onChange={e => setSettings(s => ({ ...s, permissionMode: e.target.value as AppSettings["permissionMode"] }))}
                  >
                    <option value="read-only">Read Only</option>
                    <option value="workspace-write">Workspace Write</option>
                    <option value="danger-full-access">Full Access</option>
                  </select>
                </div>
                <div className="sp-input-row" style={{ marginBottom: 8 }}>
                  <label style={{ display: "block", marginBottom: 6, color: "var(--text-dim)", fontSize: 12 }}>Approval behavior</label>
                  <select
                    className="sp-input"
                    value={settings.approvalMode}
                    onChange={e => setSettings(s => ({ ...s, approvalMode: e.target.value as AppSettings["approvalMode"] }))}
                  >
                    <option value="ask">Ask Every Request</option>
                    <option value="auto">Auto Run</option>
                  </select>
                </div>
                <div className="sp-row">
                  <span>Quick testing replies</span>
                  <button
                    className={cn("toggle-btn", settings.quickTestMode && "toggle-btn--on")}
                    onClick={() => setSettings(s => ({ ...s, quickTestMode: !s.quickTestMode }))}
                  >
                    {settings.quickTestMode ? "ON" : "OFF"}
                  </button>
                </div>
                <p className="sp-hint">When ON, prompts include a strict short-reply hint to avoid long "thinking" runs.</p>
              </section>

              <section className="sp-section">
                <h3 className="sp-section-title">Model</h3>

                <div className="model-grid">
                  {allModels.map(m => {
                    const isInstalled = ollamaModels.includes(m);
                    const isActive = settings.model === m;
                    return (
                      <div key={m} className={cn("model-card", isActive && "model-card--active")}>
                        <button
                          className="model-select-btn"
                          onClick={() => {
                            setSettings(s => ({ ...s, model: m }));
                            setAliasSource(m);
                          }}
                        >
                          <span className="model-name">{m}</span>
                          <span className={cn("model-pill", isInstalled ? "model-pill--ok" : "model-pill--missing")}>
                            {isInstalled ? "installed" : "not installed"}
                          </span>
                        </button>
                        <div className="model-actions">
                          {isInstalled ? (
                            <button
                              className="toggle-btn"
                              disabled={!!modelOpBusy}
                              onClick={() => runModelAction(`uninstall-${m}`, "uninstall_model", { name: m })}
                            >
                              Uninstall
                            </button>
                          ) : (
                            <button
                              className="sp-add-btn"
                              disabled={!!modelOpBusy}
                              onClick={() => runModelAction(`install-${m}`, "install_model", { name: m })}
                            >
                              Install
                            </button>
                          )}
                        </div>
                      </div>
                    );
                  })}
                </div>

                <h4 className="sp-subtitle">Quick Presets</h4>
                <div className="preset-grid">
                  {MODEL_PACK_PRESETS.map(preset => (
                    <div key={preset.id} className="preset-card">
                      <div className="preset-head">
                        <span className="preset-name">{preset.name}</span>
                        <button
                          className="sp-add-btn"
                          disabled={!!modelOpBusy}
                          onClick={() => runModelAction(`preset-${preset.id}`, "apply_preset", { presetId: preset.id })}
                        >
                          Install Pack
                        </button>
                      </div>
                      <p className="preset-models">{preset.models.join(" · ")}</p>
                    </div>
                  ))}
                </div>

                <h4 className="sp-subtitle">Create Alias</h4>
                <div className="alias-grid">
                  <input
                    className="sp-input"
                    value={aliasName}
                    onChange={e => setAliasName(e.target.value)}
                    placeholder="alias (e.g. angel0.1)"
                  />
                  <input
                    className="sp-input"
                    value={aliasSource}
                    onChange={e => setAliasSource(e.target.value)}
                    placeholder="source model"
                  />
                  <button
                    className="sp-add-btn"
                    disabled={!!modelOpBusy}
                    onClick={() => runModelAction(`alias-${aliasName}`, "create_model_alias", { alias: aliasName, source: aliasSource })}
                  >
                    Save Alias
                  </button>
                </div>

                {modelOpMessage && <p className="sp-hint">{modelOpMessage}</p>}
              </section>
              <section className="sp-section">
                <h3 className="sp-section-title">API Connection</h3>
                <div className="sp-input-row" style={{ marginBottom: 8 }}>
                  <label style={{ display: "block", marginBottom: 6, color: "var(--text-dim)", fontSize: 12 }}>Base URL</label>
                  <input
                    className="sp-input"
                    value={settings.baseUrl}
                    onChange={e => setSettings(s => ({ ...s, baseUrl: e.target.value }))}
                    placeholder="https://api.anthropic.com"
                  />
                </div>
                <div className="sp-input-row">
                  <label style={{ display: "block", marginBottom: 6, color: "var(--text-dim)", fontSize: 12 }}>API Key</label>
                  <input
                    className="sp-input"
                    type="password"
                    value={settings.apiKey}
                    onChange={e => setSettings(s => ({ ...s, apiKey: e.target.value }))}
                    placeholder="sk-..."
                  />
                </div>
              </section>
              <section className="sp-section">
                <h3 className="sp-section-title">Account</h3>
                <div className="sp-row">
                  <div className="sp-user-line"><User size={12} /><span>{session.username}</span></div>
                  <button className="toggle-btn" onClick={logout}><LogOut size={11} /> Logout</button>
                </div>
              </section>
              <section className="sp-section">
                <h3 className="sp-section-title">Uninstall</h3>
                <button className="sp-uninstall-btn" onClick={safeUninstall}>Safe Uninstall</button>
              </section>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

export default function App() {
  return (
    <DialogProvider>
      <AppContent />
    </DialogProvider>
  );
}
