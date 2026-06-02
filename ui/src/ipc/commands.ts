import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { Config, ServiceId, ServiceOutcomeDto } from "../types/bindings";

// ---------------------------------------------------------------------------
// Selection
// ---------------------------------------------------------------------------

export function getSelectedText(): Promise<string | null> {
  return invoke<string | null>("get_selected_text");
}

export function checkPermission(): Promise<boolean> {
  return invoke<boolean>("check_permission");
}

export function openPermissionSettings(): Promise<void> {
  return invoke<void>("open_permission_settings");
}

// ---------------------------------------------------------------------------
// Translation
// ---------------------------------------------------------------------------

export interface TranslateArgs {
  text: string;
  from?: string | null;
  to: string;
}

export function translateText(args: TranslateArgs): Promise<ServiceOutcomeDto[]> {
  return invoke<ServiceOutcomeDto[]>("translate_text", { args });
}

// ---------------------------------------------------------------------------
// Popup
// ---------------------------------------------------------------------------

export interface ShowPopupArgs {
  x: number;
  y: number;
  width: number;
  height: number;
}

export function showPopup(args: ShowPopupArgs): Promise<void> {
  return invoke<void>("show_popup", { args });
}

export function hidePopup(): Promise<void> {
  return invoke<void>("hide_popup");
}

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

export function openSettings(): Promise<void> {
  return invoke<void>("open_settings");
}

export function getConfig(): Promise<Config> {
  return invoke<Config>("get_config");
}

export function saveConfig(config: Config): Promise<void> {
  return invoke<void>("save_config", { config });
}

export interface AppInfo {
  version: string;
  commit: string;
  build_date: string;
  repo_url: string;
}

export function getAppInfo(): Promise<AppInfo> {
  return invoke<AppInfo>("get_app_info");
}

export function setApiKey(serviceId: ServiceId, apiKey: string): Promise<void> {
  return invoke<void>("set_api_key", { args: { serviceId, apiKey } });
}

export function deleteApiKey(serviceId: ServiceId): Promise<void> {
  return invoke<void>("delete_api_key", { args: { serviceId } });
}

export function hasApiKey(serviceId: ServiceId): Promise<boolean> {
  return invoke<boolean>("has_api_key", { args: { serviceId } });
}

export function updateHotkey(shortcut: string): Promise<void> {
  return invoke<void>("update_hotkey", { args: { shortcut } });
}

// ---------------------------------------------------------------------------
// Clipboard
// ---------------------------------------------------------------------------

export function copyToClipboard(text: string): Promise<void> {
  return invoke<void>("copy_to_clipboard", { args: { text } });
}

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

export function onHotkeyPressed(handler: () => void): Promise<UnlistenFn> {
  return listen("translator://hotkey-pressed", () => handler());
}
