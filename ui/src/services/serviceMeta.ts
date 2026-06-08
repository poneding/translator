import type { ServiceId } from "../types/bindings";

export interface ServiceMeta {
  id: string;
  name: string;
  iconSrc: string;
}

export const SERVICE_META: Record<ServiceId, ServiceMeta> = {
  youdao: {
    id: "youdao",
    name: "Youdao",
    iconSrc: "/service-icons/youdao.png",
  },
  deepl: {
    id: "deepl",
    name: "DeepL",
    iconSrc: "/service-icons/deepl.png",
  },
  google: {
    id: "google",
    name: "Google Translate",
    iconSrc: "/service-icons/google.png",
  },
  bing: {
    id: "bing",
    name: "Microsoft Translator",
    iconSrc: "/service-icons/bing.png",
  },
  openai: {
    id: "openai",
    name: "OpenAI Compatible",
    iconSrc: "/service-icons/openai.png",
  },
};

export function serviceMeta(id: ServiceId | string): ServiceMeta {
  const normalized = normalizeServiceId(id);
  if (normalized) return SERVICE_META[normalized];
  return {
    id,
    name: id,
    iconSrc: "",
  };
}

export function normalizeServiceId(id: ServiceId | string): ServiceId | null {
  switch (id) {
    case "youdao":
    case "deepl":
    case "google":
    case "bing":
    case "openai":
      return id;
    case "deep-l":
      return "deepl";
    case "open-ai":
      return "openai";
    default:
      return null;
  }
}
