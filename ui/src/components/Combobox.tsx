import { useEffect, useId, useRef, useState } from "react";
import { Check, ChevronDown, type LucideIcon } from "lucide-react";
import { platform as getPlatform } from "@tauri-apps/plugin-os";

export interface ComboboxOption {
  Icon?: LucideIcon;
  flag?: string;
  leading?: string;
  value: string;
  label: string;
}

export function Combobox({
  ariaLabel,
  className = "",
  label,
  onChange,
  options,
  selectedDisplay = "full",
  value,
}: {
  ariaLabel?: string;
  className?: string;
  label?: string;
  onChange: (value: string) => void;
  options: ComboboxOption[];
  selectedDisplay?: "full" | "leading";
  value: string;
}) {
  const [open, setOpen] = useState(false);
  const [showOptionFlag] = useState(() => detectHostPlatform() === "macos");
  const rootRef = useRef<HTMLDivElement | null>(null);
  const listboxId = useId();
  const selected = options.find((option) => option.value === value);

  useEffect(() => {
    if (!open) return;
    const onPointerDown = (event: PointerEvent) => {
      if (!rootRef.current?.contains(event.target as Node)) setOpen(false);
    };
    document.addEventListener("pointerdown", onPointerDown);
    return () => document.removeEventListener("pointerdown", onPointerDown);
  }, [open]);

  return (
    <div ref={rootRef} className={"relative min-w-0 " + className}>
      {label && <label className="label">{label}</label>}
      <button
        type="button"
        role="combobox"
        aria-controls={listboxId}
        aria-expanded={open}
        aria-label={ariaLabel ?? label}
        className="input flex h-8 items-center justify-between gap-2 text-left"
        onClick={() => setOpen((current) => !current)}
      >
        <ComboboxOptionContent
          display={selectedDisplay}
          option={selected}
          fallback={value}
          showFlag={showOptionFlag}
        />
        <ChevronDown
          size={15}
          aria-hidden="true"
          className="shrink-0 text-fg-subtle"
        />
      </button>
      {open && (
        <div
          id={listboxId}
          role="listbox"
          className="absolute z-50 mt-1 max-h-64 w-full overflow-y-auto rounded-md border border-border bg-bg py-1 shadow-lg"
        >
          {options.map((option) => {
            const active = option.value === value;
            return (
              <button
                key={option.value}
                type="button"
                role="option"
                aria-selected={active}
                className="flex h-8 w-full items-center justify-between gap-2 px-3 text-left text-sm text-fg hover:bg-bg-subtle"
                onClick={() => {
                  onChange(option.value);
                  setOpen(false);
                }}
              >
                <ComboboxOptionContent
                  option={option}
                  showFlag={showOptionFlag}
                />
                {active && <Check size={14} aria-hidden="true" />}
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
}

function ComboboxOptionContent({
  display = "full",
  fallback,
  option,
  showFlag,
}: {
  display?: "full" | "leading";
  fallback?: string;
  option?: ComboboxOption;
  showFlag: boolean;
}) {
  const Icon = option?.Icon;
  if (display === "leading") {
    return (
      <span className="min-w-0 flex-1 truncate text-center font-semibold">
        {option?.leading ?? option?.label ?? fallback}
      </span>
    );
  }

  return (
    <span className="flex min-w-0 flex-1 items-center gap-2">
      {Icon && (
        <Icon
          size={15}
          aria-hidden="true"
          className="shrink-0 text-fg-subtle"
        />
      )}
      {showFlag && option?.flag && (
        <span className="shrink-0 text-sm leading-none">{option.flag}</span>
      )}
      <span className="min-w-0 truncate">{option?.label ?? fallback}</span>
      {option?.leading && (
        <span className="language-code-badge">{option.leading}</span>
      )}
    </span>
  );
}

function detectHostPlatform() {
  try {
    return getPlatform();
  } catch {
    const platform = navigator.platform.toLowerCase();
    if (platform.includes("mac")) return "macos";
    if (platform.includes("win")) return "windows";
    if (platform.includes("linux")) return "linux";
    return "unknown";
  }
}
