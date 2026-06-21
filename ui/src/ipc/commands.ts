import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  Config,
  HotkeySourceDto,
  ServiceId,
  ServiceOutcomeDto,
  TranslationFinishedDto,
  TranslationOutcomeDto,
  TranslationStartedDto,
  UpdateStatusDto,
} from "../types/bindings";

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
  to?: string | null;
  request_id?: string | null;
}

export function translateText(
  args: TranslateArgs,
): Promise<ServiceOutcomeDto[]> {
  return invoke<ServiceOutcomeDto[]>("translate_text", { args });
}

export interface TranslateServiceArgs extends TranslateArgs {
  service_id: ServiceId;
}

export function translateService(
  args: TranslateServiceArgs,
): Promise<ServiceOutcomeDto> {
  return invoke<ServiceOutcomeDto>("translate_service", { args });
}

export function getTextAudioUrl(args: {
  text: string;
  language?: string | null;
}): Promise<string | null> {
  return invoke<string | null>("get_text_audio_url", { args });
}

export function createRequestId(): string {
  if (globalThis.crypto?.randomUUID) {
    return globalThis.crypto.randomUUID();
  }
  return `translation-${Date.now()}-${Math.random().toString(36).slice(2)}`;
}

export function openMainWindow(): Promise<void> {
  return invoke<void>("open_main_window");
}

export function setMainWindowAlwaysOnTop(alwaysOnTop: boolean): Promise<void> {
  return invoke<void>("set_main_window_always_on_top", {
    args: { always_on_top: alwaysOnTop },
  });
}

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

export function openSettings(): Promise<void> {
  return invoke<void>("open_settings");
}

export function restartApp(): Promise<void> {
  return invoke<void>("restart_app");
}

export function openExternalUrl(url: string): Promise<void> {
  return invoke<void>("open_external_url", { args: { url } });
}

export function getConfig(): Promise<Config> {
  return invoke<Config>("get_config");
}

export function saveConfig(config: Config): Promise<void> {
  return invoke<void>("save_config", { config });
}

export function clearHistory(): Promise<Config> {
  return invoke<Config>("clear_history");
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

export function checkUpdate(manual = true): Promise<UpdateStatusDto> {
  return invoke<UpdateStatusDto>("check_update", { args: { manual } });
}

export function downloadAndInstallUpdate(): Promise<UpdateStatusDto> {
  return invoke<UpdateStatusDto>("download_and_install_update");
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

export function readClipboard(): Promise<string> {
  return invoke<string>("read_clipboard");
}

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

export function onHotkeySource(
  handler: (payload: HotkeySourceDto) => void,
): Promise<UnlistenFn> {
  return listen<HotkeySourceDto>("translator://hotkey-source", ({ payload }) =>
    handler(payload),
  );
}

export function onOpenSettingsRequested(
  handler: () => void,
): Promise<UnlistenFn> {
  return listen("translator://open-settings", () => handler());
}

export function onCheckUpdateRequested(
  handler: () => void,
): Promise<UnlistenFn> {
  return listen("translator://check-update", () => handler());
}

export function onOpenMainRequested(handler: () => void): Promise<UnlistenFn> {
  return listen("translator://open-main", () => handler());
}

export function onTranslationStarted(
  handler: (payload: TranslationStartedDto) => void,
): Promise<UnlistenFn> {
  return listen<TranslationStartedDto>(
    "translator://translation-started",
    ({ payload }) => handler(payload),
  );
}

export function onTranslationOutcome(
  handler: (payload: TranslationOutcomeDto) => void,
): Promise<UnlistenFn> {
  return listen<TranslationOutcomeDto>(
    "translator://translation-outcome",
    ({ payload }) => handler(payload),
  );
}

export function onTranslationFinished(
  handler: (payload: TranslationFinishedDto) => void,
): Promise<UnlistenFn> {
  return listen<TranslationFinishedDto>(
    "translator://translation-finished",
    ({ payload }) => handler(payload),
  );
}

export function onConfigUpdated(
  handler: (payload: Config) => void,
): Promise<UnlistenFn> {
  return listen<Config>("translator://config-updated", ({ payload }) =>
    handler(payload),
  );
}

export function onUpdateStatus(
  handler: (payload: UpdateStatusDto) => void,
): Promise<UnlistenFn> {
  return listen<UpdateStatusDto>("translator://update-status", ({ payload }) =>
    handler(payload),
  );
}
