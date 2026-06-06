import { invoke } from "@tauri-apps/api/core";
import type {
  BackupEntry,
  BackupOpResult,
  HistoryEntry,
  RestorePointStatus,
} from "../types/backup";

export const listBackups = () =>
  invoke<BackupEntry[]>("list_backups");

export const createRegistryBackup = (label: string, registryKey: string) =>
  invoke<BackupOpResult>("create_registry_backup", { label, registryKey });

export const restoreRegistryFile = (id: string) =>
  invoke<BackupOpResult>("restore_registry_file", { id });

export const deleteBackup = (id: string) =>
  invoke<BackupOpResult>("delete_backup", { id });

export const checkRestorePointStatus = () =>
  invoke<RestorePointStatus>("check_restore_point_status");

export const createRestorePoint = (description: string) =>
  invoke<BackupOpResult>("create_restore_point", { description });

export const listHistory = () =>
  invoke<HistoryEntry[]>("list_history");

export const clearHistory = () =>
  invoke<BackupOpResult>("clear_history");
