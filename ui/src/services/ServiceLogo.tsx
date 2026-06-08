import type { ImgHTMLAttributes } from "react";
import type { ServiceId } from "../types/bindings";
import { serviceMeta } from "./serviceMeta";

export function ServiceLogo({
  serviceId,
  className = "h-5 w-5",
  alt,
  ...props
}: {
  serviceId: ServiceId | string;
  className?: string;
  alt?: string;
} & Omit<ImgHTMLAttributes<HTMLImageElement>, "src" | "alt">) {
  const meta = serviceMeta(serviceId);
  if (!meta.iconSrc) {
    return (
      <span
        {...props}
        aria-label={alt ?? meta.name}
        className={`${className} inline-flex shrink-0 items-center justify-center rounded-sm border border-border bg-bg-subtle text-[10px] font-semibold text-fg-subtle`}
      >
        {meta.name.slice(0, 1).toUpperCase()}
      </span>
    );
  }

  return (
    <img
      {...props}
      src={meta.iconSrc}
      alt={alt ?? meta.name}
      className={`${className} shrink-0 rounded-sm object-contain`}
      draggable={false}
    />
  );
}
