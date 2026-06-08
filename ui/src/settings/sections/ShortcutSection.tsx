import { useEffect, useRef, useState, type KeyboardEvent } from "react";
import { Keyboard } from "lucide-react";
import { useConfigStore } from "../../stores/config";
import { useT } from "../../i18n";
import * as api from "../../ipc/commands";

// BH-10.2: tauri-plugin-global-shortcut string syntax.
//   <mod>(+<mod>)*+<key>
//   mod ∈ {CmdOrCtrl, Cmd, Ctrl, Super, Shift, Alt, Option, Meta, Win}
//   key ∈ {A-Z, 0-9, Space, Enter, Return, Escape, Esc, Tab, Backspace, Delete, Del}
const HOTKEY_RE =
  /^(?:(?:CmdOrCtrl|Cmd|Ctrl|Super|Shift|Alt|Option|Meta|Win)\+)+(?:[A-Za-z]|[0-9]|Space|Enter|Return|Escape|Esc|Tab|Backspace|Delete|Del)$/;

export function ShortcutSection() {
  const { config, save } = useConfigStore();
  const t = useT();
  const [err, setErr] = useState<string | null>(null);
  const [shortcut, setShortcut] = useState("");
  const [recording, setRecording] = useState(false);
  const inputRef = useRef<HTMLInputElement | null>(null);

  useEffect(() => {
    setShortcut(config?.shortcut ?? "");
  }, [config?.shortcut]);

  if (!config) return null;

  // BH-1.5: the backend sets config.hotkey_registration_failed when the OS
  // denies registration (e.g. conflict with another app). The launch code
  // resets the shortcut to the default on next start, but the flag is
  // surfaced here so the user knows what happened.
  const banner = config.hotkey_registration_failed;

  const applyShortcut = async (value: string) => {
    const next = value.trim();
    if (!HOTKEY_RE.test(next)) {
      setErr(t("settings-shortcut-invalid"));
      return;
    }
    if (next === config.shortcut) {
      setErr(null);
      setRecording(false);
      return;
    }
    try {
      await api.updateHotkey(next);
      await save({ ...config, shortcut: next });
      setShortcut(next);
      setErr(null);
      setRecording(false);
    } catch (ex) {
      setErr(String(ex));
      setRecording(false);
      try {
        const fresh = await api.getConfig();
        if (fresh) {
          await save({ ...config, ...fresh, shortcut: config.shortcut });
          setShortcut(config.shortcut);
        }
      } catch {
        /* best-effort */
      }
    }
  };

  return (
    <div className="space-y-2">
      {banner && (
        <div className="rounded-md border border-red-500 bg-red-500/10 p-2 text-xs text-red-500">
          {t("settings-shortcut-registration-denied")}
        </div>
      )}
      <label className="label">{t("settings-shortcut-label")}</label>
      <div className="flex gap-2">
        <input
          ref={inputRef}
          className={"input min-w-0 " + (err ? "border-red-500" : "")}
          aria-label={t("settings-shortcut-aria")}
          value={shortcut}
          placeholder={platformDefaultShortcut()}
          onChange={(event) => setShortcut(event.target.value)}
          onBlur={() => {
            if (!recording) void applyShortcut(shortcut);
          }}
          onKeyDown={(event) => {
            if (!recording) return;
            event.preventDefault();
            event.stopPropagation();
            const captured = shortcutFromEvent(event);
            if (!captured) return;
            setShortcut(captured);
            void applyShortcut(captured);
          }}
        />
        <button
          className={"btn " + (recording ? "btn-primary" : "")}
          type="button"
          onClick={() => {
            setErr(null);
            setRecording(true);
            inputRef.current?.focus();
          }}
        >
          <Keyboard size={15} aria-hidden="true" />
          {recording
            ? t("settings-shortcut-recording")
            : t("settings-shortcut-record")}
        </button>
      </div>
      {err && <p className="text-xs text-red-500">{err}</p>}
      {!err && (
        <p
          className={
            "text-xs " + (recording ? "text-accent" : "text-fg-subtle")
          }
        >
          {t("settings-shortcut-hint")}
        </p>
      )}
    </div>
  );
}

function shortcutFromEvent(
  event: KeyboardEvent<HTMLInputElement>,
): string | null {
  const key = normalizeKey(event.key);
  if (!key) return null;

  const modifiers: string[] = [];
  if (event.metaKey) modifiers.push("Cmd");
  if (event.ctrlKey) modifiers.push("Ctrl");
  if (event.altKey) modifiers.push("Alt");
  if (event.shiftKey) modifiers.push("Shift");

  if (modifiers.length === 0) return null;
  return [...modifiers, key].join("+");
}

function normalizeKey(key: string): string | null {
  const lower = key.toLowerCase();
  if (["control", "shift", "alt", "meta", "os"].includes(lower)) return null;
  if (lower === " ") return "Space";
  if (lower === "escape") return "Escape";
  if (lower === "enter" || lower === "return") return "Enter";
  if (lower === "backspace") return "Backspace";
  if (lower === "delete" || lower === "del") return "Delete";
  if (lower === "tab") return "Tab";
  if (/^[a-z]$/.test(lower)) return lower.toUpperCase();
  if (/^[0-9]$/.test(lower)) return lower;
  return null;
}

function platformDefaultShortcut(): string {
  const platform = navigator.platform.toLowerCase();
  return platform.includes("mac") ? "Cmd+T" : "Alt+T";
}
