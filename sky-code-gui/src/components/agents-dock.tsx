/**
 * AgentsDock — SkyCode multi-agent selector
 *
 * Wraps MessageDock with SkyCode's 4 specialist agents + a Central broadcast channel.
 * Colors align with the SkyCode design system (SKYCODE-UI-UX-SKILL.md section 2).
 *
 * Agent roster (slice(1,-1) visible = indices 1-4):
 *   Index 0  — sparkle btn position (not shown as character)
 *   Index 1  — Manager  (Sky Blue  #38bdf8)   ← orchestrates specialists
 *   Index 2  — SQL      (Amber     #fbbf24)   ← database schemas, queries
 *   Index 3  — Backend  (Emerald   #34d399)   ← API routes, services
 *   Index 4  — Frontend (Violet    #a78bfa)   ← UI components, state
 *   Index 5  — menu btn position (not shown as character)
 *
 * The ✨ sparkle button = "Central" — sends to the general sky runtime (all agents see it).
 */

import { MessageDock, Character } from "@/components/ui/message-dock";

export interface AgentMessage {
  text: string;
  /** agent name, e.g. "Manager", "SQL", "Backend", "Frontend", or "Central" */
  agent: string;
  agentIndex: number;
  /** true when the sparkle/central button triggers broadcast mode */
  broadcast?: boolean;
  timestamp: Date;
}

interface AgentsDockProps {
  onMessage?: (msg: AgentMessage) => void;
  /** Override agent online status from live agent manifests */
  agentStatus?: Partial<Record<"Manager" | "SQL" | "Backend" | "Frontend", boolean>>;
  theme?: "light" | "dark";
}

/**
 * 6-item array so slice(1,-1) exposes exactly 4 clickable agents.
 * Colors use SkyCode brand palette.
 */
const buildAgents = (
  status: Partial<Record<string, boolean>>,
): Character[] => [
  // Index 0: sparkle button slot (decorative ✨ is hardcoded in dock)
  { emoji: "⚡", name: "Central", online: true },

  // Index 1 → 4: visible specialist agents
  {
    emoji: "🧠",
    name: "Manager",
    online: status["Manager"] ?? true,
    backgroundColor: "bg-sky-400",
    gradientColors: "#38bdf8, #e0f2fe", // Sky Blue — SkyCode primary
  },
  {
    emoji: "🗄️",
    name: "SQL",
    online: status["SQL"] ?? true,
    backgroundColor: "bg-amber-400",
    gradientColors: "#fbbf24, #fef3c7", // Amber — warning/data tone
  },
  {
    emoji: "⚙️",
    name: "Backend",
    online: status["Backend"] ?? true,
    backgroundColor: "bg-emerald-400",
    gradientColors: "#34d399, #d1fae5", // Emerald — success/API tone
  },
  {
    emoji: "🎨",
    name: "Frontend",
    online: status["Frontend"] ?? false,
    backgroundColor: "bg-violet-400",
    gradientColors: "#a78bfa, #ede9fe", // Violet — UI/creative tone
  },

  // Index 5: menu button slot (not rendered as character)
  { emoji: "+", name: "More", online: false },
];

export function AgentsDock({
  onMessage,
  agentStatus = {},
  theme = "dark",
}: AgentsDockProps) {
  const agents = buildAgents(agentStatus);

  const handleMessageSend = (
    message: string,
    character: Character,
    characterIndex: number,
  ) => {
    onMessage?.({
      text: message,
      agent: character.name,
      agentIndex: characterIndex,
      broadcast: false,
      timestamp: new Date(),
    });
  };

  const handleCharacterSelect = (character: Character, index: number) => {
    // Optional: track which agent is focused for status display
    console.debug("[AgentsDock] selected:", character.name, "at index", index);
  };

  return (
    <MessageDock
      characters={agents}
      onMessageSend={handleMessageSend}
      onCharacterSelect={handleCharacterSelect}
      expandedWidth={480}
      theme={theme}
      placeholder={(name) => `Message ${name} agent...`}
      enableAnimations={true}
      closeOnSend={true}
      closeOnEscape={true}
      closeOnClickOutside={true}
      autoFocus={true}
      showSparkleButton={true}
      showMenuButton={true}
    />
  );
}
