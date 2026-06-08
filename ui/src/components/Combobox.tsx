import { useEffect, useId, useRef, useState } from "react";
import { Check, ChevronDown } from "lucide-react";

export interface ComboboxOption {
  value: string;
  label: string;
}

export function Combobox({
  ariaLabel,
  className = "",
  label,
  onChange,
  options,
  value,
}: {
  ariaLabel?: string;
  className?: string;
  label?: string;
  onChange: (value: string) => void;
  options: ComboboxOption[];
  value: string;
}) {
  const [open, setOpen] = useState(false);
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
        <span className="min-w-0 truncate">
          {selected?.label ?? value}
        </span>
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
          className="absolute z-30 mt-1 max-h-64 w-full overflow-y-auto rounded-md border border-border bg-bg py-1 shadow-lg"
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
                <span className="min-w-0 truncate">{option.label}</span>
                {active && <Check size={14} aria-hidden="true" />}
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
}
