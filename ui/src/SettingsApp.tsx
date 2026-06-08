import { useEffect } from "react";
import { useConfigStore } from "./stores/config";
import { useT } from "./i18n";
import { useTheme } from "./hooks/useTheme";
import { GeneralSection } from "./settings/sections/GeneralSection";
import { ServicesSection } from "./settings/sections/ServicesSection";
import { ShortcutSection } from "./settings/sections/ShortcutSection";
import { AppearanceSection } from "./settings/sections/AppearanceSection";
import { AboutSection } from "./settings/sections/AboutSection";
import * as api from "./ipc/commands";
import { getCurrentWindow } from "@tauri-apps/api/window";

type SectionId = "general" | "services" | "shortcut" | "appearance" | "about";

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

  const sections: { id: SectionId; label: string; title: string }[] = [
    { id: "general", label: t("settings-nav-general"), title: t("settings-nav-general") },
    { id: "services", label: t("settings-nav-services"), title: t("settings-services-title") },
    { id: "shortcut", label: t("settings-nav-shortcut"), title: t("settings-nav-shortcut") },
    { id: "appearance", label: t("settings-nav-appearance"), title: t("settings-nav-appearance") },
    { id: "about", label: t("settings-nav-about"), title: t("settings-nav-about") },
  ];

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

  return (
    <div className="flex h-full min-h-0 bg-bg text-fg">
      <aside className="w-48 shrink-0 border-r border-border bg-bg-subtle p-3">
        <h1 className="px-2 pb-2 text-sm font-semibold text-fg">
          {t("app-name", null, "Translator")}
        </h1>
        <nav className="flex flex-col gap-0.5">
          {sections.map((section) => (
            <a
              key={section.id}
              href={`#${section.id}`}
              className="rounded-md px-2 py-1 text-sm text-fg-subtle hover:bg-border hover:text-fg"
            >
              {section.label}
            </a>
          ))}
        </nav>
      </aside>
      <main className="min-w-0 flex-1 overflow-y-auto p-6">
        <div className="mx-auto max-w-3xl space-y-6">
          <SettingsSection id="general" title={sections[0].title}>
            <GeneralSection />
          </SettingsSection>
          <SettingsSection id="services" title={sections[1].title}>
            <ServicesSection />
          </SettingsSection>
          <SettingsSection id="shortcut" title={sections[2].title}>
            <ShortcutSection />
          </SettingsSection>
          <SettingsSection id="appearance" title={sections[3].title}>
            <AppearanceSection />
          </SettingsSection>
          <SettingsSection id="about" title={sections[4].title}>
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
  children,
}: {
  id: string;
  title: string;
  children: React.ReactNode;
}) {
  return (
    <section id={id} className="space-y-3">
      <h2 className="text-lg font-semibold">{title}</h2>
      <div className="card">{children}</div>
    </section>
  );
}
