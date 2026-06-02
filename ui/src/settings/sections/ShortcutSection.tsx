import { useState } from "react";
import { useConfigStore } from "../../stores/config";
import * as api from "../../ipc/commands";

// BH-10.2: tauri-plugin-global-shortcut string syntax.
//   <mod>(+<mod>)*+<key>
//   mod ∈ {CmdOrCtrl, Cmd, Ctrl, Super, Shift, Alt, Option, Meta, Win}
//   key ∈ {A-Z, 0-9, Space, Enter, Return, Escape, Esc, Tab, Backspace, Delete, Del}
const HOTKEY_RE = /^(?:(?:CmdOrCtrl|Cmd|Ctrl|Super|Shift|Alt|Option|Meta|Win)\+)+(?:[A-Za-z]|[0-9]|Space|Enter|Return|Escape|Esc|Tab|Backspace|Delete|Del)$/;

export function ShortcutSection() {
  const { config, save } = useConfigStore();
  const [err, setErr] = useState<string | null>(null);
  if (!config) return null;

  // BH-1.5: the backend sets config.hotkey_registration_failed when the OS
  // denies registration (e.g. conflict with another app). The launch code
  // resets the shortcut to the default on next start, but the flag is
  // surfaced here so the user knows what happened.
  const banner = config.hotkey_registration_failed;

  return (
    <div className="space-y-2">
      {banner && (
        <div className="rounded-md border border-red-500 bg-red-500/10 p-2 text-xs text-red-500">
          The OS denied hotkey registration (likely a conflict). The default
          shortcut will be restored on next launch.
        </div>
      )}
      <label className="label">Global hotkey</label>
      <input
        className={"input " + (err ? "border-red-500" : "")}
        aria-label="Global hotkey (Tauri global-shortcut syntax)"
        defaultValue={config.shortcut}
        placeholder="CmdOrCtrl+Shift+D"
        onBlur={async (e) => {
          const v = e.target.value.trim();
          if (!HOTKEY_RE.test(v)) {
            setErr(
              'Invalid syntax. Use "<modifiers>+<key>" — e.g. "CmdOrCtrl+Shift+D", "Alt+T".',
            );
            return;
          }
          if (v === config.shortcut) {
            setErr(null);
            return;
          }
          // BH-10.3: re-register the live global shortcut before
          // persisting, so a bad change never leaves the app hotkey-less.
          try {
            await api.updateHotkey(v);
            await save({ ...config, shortcut: v });
            setErr(null);
          } catch (ex) {
            setErr(String(ex));
            // The backend will have set hotkey_registration_failed=true;
            // refresh the config so the banner appears immediately.
            if (api.getConfig) {
              try {
                const fresh = await api.getConfig();
                if (fresh) {
                  await save({ ...config, ...fresh, shortcut: config.shortcut });
                }
              } catch {
                /* best-effort */
              }
            }
          }
        }}
      />
      {err && <p className="text-xs text-red-500">{err}</p>}
      {!err && (
        <p className="text-xs text-fg-subtle">
          Tauri global-shortcut syntax. Modifiers: CmdOrCtrl, Cmd, Ctrl, Super,
          Shift, Alt, Option, Meta, Win. Key: A–Z, 0–9, Space, Enter, Escape,
          Tab, Backspace, Delete. Saved on blur; the live hotkey is re-registered
          immediately.
        </p>
      )}
    </div>
  );
}
