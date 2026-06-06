export type StartupSource =
  | "hkcu_run"
  | "hklm_run"
  | "user_startup"
  | "common_startup";

export type StartupStatus = "enabled" | "disabled";

export interface StartupApp {
  id: string;
  name: string;
  command: string;
  source: StartupSource;
  source_display: string;
  status: StartupStatus;
  is_sensitive: boolean;
}

export interface StartupOpResult {
  success: boolean;
  message: string;
  data: StartupApp | null;
  error: string | null;
}

export const SOURCE_LABELS: Record<StartupSource, string> = {
  hkcu_run:        "HKCU\\Run",
  hklm_run:        "HKLM\\Run",
  user_startup:    "User Startup",
  common_startup:  "Common Startup",
};

/** Sources that require elevated privileges to modify */
export const ELEVATED_SOURCES = new Set<StartupSource>(["hklm_run", "common_startup"]);
