import {
  Rocket,
  Archive,
  Gamepad2,
  Wrench,
  ShieldCheck,
  ShieldAlert,
  Settings,
} from "lucide-react";
import { cn } from "../lib/cn";
import { useAppStore } from "../store/useAppStore";
import type { TweakCategory } from "../types";

export interface SidebarTool {
  id: string;
  name: string;
  icon: string;
}

const TOOL_ICONS: Record<string, React.ReactNode> = {
  "startup-apps":   <Rocket  size={13} strokeWidth={1.8} />,
  "backup-restore": <Archive size={13} strokeWidth={1.8} />,
  "gaming-tools":   <Wrench  size={13} strokeWidth={1.8} />,
};

interface SidebarProps {
  tools: SidebarTool[];
  categories: TweakCategory[];
  activePage: string;
  onSelectPage: (id: string) => void;
  tweakCounts: Record<string, number>;
  appliedCounts: Record<string, number>;
  isAdmin: boolean;
  onRestartAsAdmin: () => void;
}

export function Sidebar({
  tools,
  categories,
  activePage,
  onSelectPage,
  tweakCounts,
  appliedCounts,
  isAdmin,
  onRestartAsAdmin,
}: SidebarProps) {
  const appVersion = useAppStore((s) => s.appVersion);

  return (
    <aside className="sidebar">
      {/* ── Header ─────────────────────────────────────────────── */}
      <div className="sidebar-header">
        <Settings size={14} strokeWidth={2} style={{ color: "var(--accent)", flexShrink: 0 }} />
        <span className="sidebar-title">VOptimizer</span>
      </div>

      {/* ── Admin badge ────────────────────────────────────────── */}
      <div
        className={cn(
          "sidebar-admin-badge",
          isAdmin ? "sidebar-admin-badge--admin" : "sidebar-admin-badge--limited"
        )}
      >
        {isAdmin ? (
          <>
            <ShieldCheck size={11} strokeWidth={2} className="sidebar-admin-icon" />
            <span className="sidebar-admin-text">Admin mode</span>
          </>
        ) : (
          <>
            <ShieldAlert size={11} strokeWidth={2} className="sidebar-admin-icon" />
            <span className="sidebar-admin-text">Limited mode</span>
            <button
              className="sidebar-restart-admin-btn"
              onClick={onRestartAsAdmin}
              title="Restart VOptimizer with administrator privileges"
            >
              Restart as Admin
            </button>
          </>
        )}
      </div>

      {/* ── Nav ────────────────────────────────────────────────── */}
      <nav className="sidebar-nav">
        <p className="sidebar-section-label">TOOLS</p>
        {tools.map((tool) => (
          <button
            key={tool.id}
            className={cn("sidebar-item", activePage === tool.id && "sidebar-item--active")}
            onClick={() => onSelectPage(tool.id)}
          >
            <span className="sidebar-item-icon">
              {TOOL_ICONS[tool.id] ?? tool.icon}
            </span>
            <span className="sidebar-item-name">{tool.name}</span>
          </button>
        ))}

        <p className="sidebar-section-label" style={{ marginTop: "16px" }}>
          MINECRAFT
        </p>
        <button
          className={cn("sidebar-item", activePage === "minecraft" && "sidebar-item--active")}
          onClick={() => onSelectPage("minecraft")}
        >
          <span className="sidebar-item-icon">
            <Gamepad2 size={13} strokeWidth={1.8} />
          </span>
          <span className="sidebar-item-name">Minecraft</span>
        </button>

        <p className="sidebar-section-label" style={{ marginTop: "16px" }}>
          TWEAKS
        </p>
        {categories.map((cat) => {
          const total = tweakCounts[cat.id] ?? 0;
          const applied = appliedCounts[cat.id] ?? 0;
          const isActive = activePage === cat.id;
          return (
            <button
              key={cat.id}
              className={cn("sidebar-item", isActive && "sidebar-item--active")}
              onClick={() => onSelectPage(cat.id)}
            >
              <span className="sidebar-item-icon">{cat.icon}</span>
              <span className="sidebar-item-name">{cat.name}</span>
              <span className="sidebar-item-badge">
                {applied > 0 && (
                  <span className="badge badge--applied">{applied}</span>
                )}
                <span className="badge badge--total">{total}</span>
              </span>
            </button>
          );
        })}
      </nav>

      <div className="sidebar-footer">
        <span className="sidebar-version">{appVersion}</span>
      </div>
    </aside>
  );
}
