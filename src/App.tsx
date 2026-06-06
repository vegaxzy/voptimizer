import { useState, useMemo, useCallback } from "react";
import "./App.css";
import { CATEGORIES, ALL_TWEAKS } from "./data/tweaks";
import { Sidebar } from "./components/Sidebar";
import type { SidebarTool } from "./components/Sidebar";
import { CategoryPage } from "./components/CategoryPage";
import { StartupAppsPage } from "./pages/StartupAppsPage";
import { BackupRestorePage } from "./pages/BackupRestorePage";
import { MinecraftPage } from "./pages/MinecraftPage";
import { ToolsPage } from "./pages/ToolsPage";
import { useTweaks } from "./hooks/useTweaks";
import { useAppStore } from "./store/useAppStore";

const TOOLS: SidebarTool[] = [
  { id: "startup-apps",   name: "Startup Apps",    icon: "🚀" },
  { id: "backup-restore", name: "Backup & Restore", icon: "🛡️" },
  { id: "gaming-tools",   name: "Gaming Tools",     icon: "🔧" },
];

const TOOL_IDS = new Set(TOOLS.map((t) => t.id));

function App() {
  const [activePage, setActivePage] = useState<string>(CATEGORIES[0].id);
  const setCurrentPage = useAppStore((s) => s.setCurrentPage);

  const handleSelectPage = useCallback(
    (id: string) => {
      setActivePage(id);
      setCurrentPage(id);
    },
    [setCurrentPage]
  );

  const {
    tweakStates,
    hasNvidia,
    hasAmd,
    isAdmin,
    logs,
    pendingApplyId,
    requestApply,
    confirmApply,
    cancelApply,
    revert,
    clearLogs,
    applyPreset,
    restartAsAdmin,
  } = useTweaks();

  const isTool = TOOL_IDS.has(activePage);
  const isMinecraft = activePage === "minecraft";
  const isGamingTools = activePage === "gaming-tools";
  const activeCategory = (isTool || isMinecraft) ? null : CATEGORIES.find((c) => c.id === activePage) ?? null;

  const tweakCounts = useMemo(
    () =>
      Object.fromEntries(
        CATEGORIES.map((c) => [
          c.id,
          ALL_TWEAKS.filter((t) => t.category === c.id).length,
        ])
      ),
    []
  );

  const appliedCounts = useMemo(
    () =>
      Object.fromEntries(
        CATEGORIES.map((c) => [
          c.id,
          ALL_TWEAKS.filter((t) => t.category === c.id && tweakStates[t.id]?.isApplied).length,
        ])
      ),
    [tweakStates]
  );

  return (
    <div className="app-shell">
      <Sidebar
        tools={TOOLS}
        categories={CATEGORIES}
        activePage={activePage}
        onSelectPage={handleSelectPage}
        tweakCounts={tweakCounts}
        appliedCounts={appliedCounts}
        isAdmin={isAdmin}
        onRestartAsAdmin={restartAsAdmin}
      />

      <div className="main-area">
        {activePage === "startup-apps" ? (
          <StartupAppsPage isAdmin={isAdmin} onRestartAsAdmin={restartAsAdmin} />
        ) : activePage === "backup-restore" ? (
          <BackupRestorePage isAdmin={isAdmin} onRestartAsAdmin={restartAsAdmin} />
        ) : isGamingTools ? (
          <ToolsPage isAdmin={isAdmin} onRestartAsAdmin={restartAsAdmin} />
        ) : activePage === "minecraft" ? (
          <MinecraftPage
            tweakStates={tweakStates}
            hasNvidia={hasNvidia}
            hasAmd={hasAmd}
            isAdmin={isAdmin}
            onRequestApply={requestApply}
            onRevert={revert}
            onConfirmApply={confirmApply}
            onCancelApply={cancelApply}
            pendingApplyId={pendingApplyId}
            applyPreset={applyPreset}
            onRestartAsAdmin={restartAsAdmin}
          />
        ) : activeCategory ? (
          <CategoryPage
            category={activeCategory}
            tweakStates={tweakStates}
            hasNvidia={hasNvidia}
            hasAmd={hasAmd}
            isAdmin={isAdmin}
            logs={logs}
            pendingApplyId={pendingApplyId}
            onRequestApply={requestApply}
            onRevert={revert}
            onConfirmApply={confirmApply}
            onCancelApply={cancelApply}
            onClearLogs={clearLogs}
            onRestartAsAdmin={restartAsAdmin}
          />
        ) : null}
      </div>
    </div>
  );
}

export default App;
