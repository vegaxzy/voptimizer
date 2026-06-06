export interface BackupEntry {
  id: string;
  timestamp: number;     // Unix ms
  label: string;
  registry_key: string;
  file_path: string;
  size_bytes: number;
}

export interface HistoryEntry {
  id: string;
  timestamp: number;     // Unix ms
  /** "disable_startup" | "enable_startup" | "apply_tweak" | "revert_tweak"
   *  | "create_backup" | "restore_backup" | "delete_backup" | "create_restore_point" */
  action: string;
  /** "startup" | "tweak" | "backup" */
  category: string;
  target: string;
  success: boolean;
  message: string;
}

export interface BackupOpResult {
  success: boolean;
  message: string;
  data: BackupEntry | null;
  error: string | null;
}

export interface RestorePointStatus {
  enabled: boolean;
  message: string;
}

export const PRESET_REGISTRY_KEYS: { label: string; key: string }[] = [
  {
    label: "Startup — Current User",
    key: "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
  },
  {
    label: "Startup — All Users",
    key: "HKLM\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
  },
  {
    label: "VOptimizer Data",
    key: "HKCU\\Software\\VOptimizer",
  },
];

export const ACTION_LABELS: Record<string, string> = {
  disable_startup:       "Disable Startup",
  enable_startup:        "Enable Startup",
  apply_tweak:           "Apply Tweak",
  revert_tweak:          "Revert Tweak",
  create_backup:         "Create Backup",
  restore_backup:        "Restore Backup",
  delete_backup:         "Delete Backup",
  create_restore_point:  "Create Restore Point",
};

export const CATEGORY_LABELS: Record<string, string> = {
  startup: "Startup",
  tweak:   "Tweak",
  backup:  "Backup",
};
