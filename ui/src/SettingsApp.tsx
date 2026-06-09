import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  ArrowLeft,
  Globe2,
  Info,
  Keyboard,
  Languages,
  Palette,
  SlidersHorizontal,
  type LucideIcon,
} from "lucide-react";
import { useEffect } from "react";
import { useTheme } from "./hooks/useTheme";
import { useT } from "./i18n";
import * as api from "./ipc/commands";
import { AboutSection } from "./settings/sections/AboutSection";
import { AppearanceSection } from "./settings/sections/AppearanceSection";
import { GeneralSection } from "./settings/sections/GeneralSection";
import { ProxySection } from "./settings/sections/ProxySection";
import { ServicesSection } from "./settings/sections/ServicesSection";
import { ShortcutSection } from "./settings/sections/ShortcutSection";
import { useConfigStore } from "./stores/config";

type SectionId = "general" | "proxy" | "services" | "shortcut" | "appearance" | "about";

export function SettingsApp() {
  const { config, load, loading, error, setConfig } = useConfigStore();
  const t = useT();

  useTheme((config?.general.theme as "system" | "light" | "dark" | undefined) ?? "system");

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    const win = getCurrentWindow();
    const unlistenPromise = win.onCloseRequested((event) => {
      event.preventDefault();
      void win.hide();
    });
    return () => {
      void unlistenPromise.then((unlisten) => unlisten());
    };
  }, []);

  useEffect(() => {
    const unlistenPromise = api.onConfigUpdated((nextConfig) => {
      setConfig(nextConfig);
    });
    return () => {
      void unlistenPromise.then((unlisten) => unlisten());
    };
  }, [setConfig]);

  if (loading && !config) {
    return <div className="p-8 text-fg-subtle">{t("common-loading", null, "Loading...")}</div>;
  }
  if (error) {
    return (
      <div className="p-8 text-red-500">
        {t("common-load-config-failed", { msg: error }, `Failed to load config: ${error}`)}
      </div>
    );
  }
  if (!config) return null;

  return <SettingsView />;
}

export function SettingsView({ onBack }: { onBack?: () => void }) {
  const t = useT();
  const sections: { id: SectionId; label: string; title: string; Icon: LucideIcon }[] = [
    {
      id: "general",
      label: t("settings-nav-general"),
      title: t("settings-nav-general"),
      Icon: SlidersHorizontal,
    },
    {
      id: "services",
      label: t("settings-nav-services"),
      title: t("settings-services-title"),
      Icon: Languages,
    },
    {
      id: "shortcut",
      label: t("settings-nav-shortcut"),
      title: t("settings-nav-shortcut"),
      Icon: Keyboard,
    },
    {
      id: "appearance",
      label: t("settings-nav-appearance"),
      title: t("settings-nav-appearance"),
      Icon: Palette,
    },
    {
      id: "proxy",
      label: t("settings-nav-proxy"),
      title: t("settings-nav-proxy"),
      Icon: Globe2,
    },
    {
      id: "about",
      label: t("settings-nav-about"),
      title: t("settings-nav-about"),
      Icon: Info,
    },
  ];

  return (
    <div className="flex h-full min-h-0 bg-bg text-fg">
      <aside className="w-fit min-w-[8.5rem] max-w-[11rem] shrink-0 border-r border-border bg-bg-subtle p-3">
        {onBack ? (
          <button className="btn btn-ghost mb-3 w-full px-2 justify-start whitespace-nowrap" onClick={onBack}>
            <ArrowLeft size={15} aria-hidden="true" />
            {t("settings-back-main", null, "Back to main")}
          </button>
        ) : (
          <h1 className="px-2 pb-2 text-sm font-semibold text-fg">
            {t("app-name", null, "Translator")}
          </h1>
        )}
        <nav className="flex flex-col gap-0.5">
          {sections.map((section) => (
            <a
              key={section.id}
              href={`#${section.id}`}
              className="inline-flex items-center gap-2 whitespace-nowrap rounded-md px-2 py-1 text-sm text-fg-subtle hover:bg-border hover:text-fg"
            >
              <section.Icon size={15} aria-hidden="true" className="shrink-0" />
              {section.label}
            </a>
          ))}
        </nav>
      </aside>
      <main className="min-w-0 flex-1 overflow-y-auto p-6">
        <div className="mx-auto max-w-3xl space-y-6">
          <SettingsSection id="general" title={sections[0].title} Icon={sections[0].Icon}>
            <GeneralSection />
          </SettingsSection>
          <SettingsSection id="services" title={sections[1].title} Icon={sections[1].Icon}>
            <ServicesSection />
          </SettingsSection>
          <SettingsSection id="shortcut" title={sections[2].title} Icon={sections[2].Icon}>
            <ShortcutSection />
          </SettingsSection>
          <SettingsSection id="appearance" title={sections[3].title} Icon={sections[3].Icon}>
            <AppearanceSection />
          </SettingsSection>
          <SettingsSection id="proxy" title={sections[4].title} Icon={sections[4].Icon}>
            <ProxySection />
          </SettingsSection>
          <SettingsSection id="about" title={sections[5].title} Icon={sections[5].Icon}>
            <AboutSection />
          </SettingsSection>
        </div>
      </main>
    </div>
  );
}

function SettingsSection({
  id,
  title,
  Icon,
  children,
}: {
  id: string;
  title: string;
  Icon: LucideIcon;
  children: React.ReactNode;
}) {
  return (
    <section id={id} className="space-y-3">
      <h2 className="flex items-center gap-2 text-lg font-semibold">
        <Icon size={18} aria-hidden="true" className="shrink-0 text-fg-subtle" />
        {title}
      </h2>
      <div className="card">{children}</div>
    </section>
  );
}
