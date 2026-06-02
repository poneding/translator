import { useEffect } from "react";
import { useConfigStore } from "./stores/config";
import { useT } from "./i18n";
import { useTheme } from "./hooks/useTheme";
import { GeneralSection } from "./settings/sections/GeneralSection";
import { ServicesSection } from "./settings/sections/ServicesSection";
import { ShortcutSection } from "./settings/sections/ShortcutSection";
import { AppearanceSection } from "./settings/sections/AppearanceSection";
import { AboutSection } from "./settings/sections/AboutSection";

type Section = "general" | "services" | "shortcut" | "appearance" | "about";

export function App() {
  const { config, load, loading, error } = useConfigStore();
  const t = useT();
  const section: Section = "general"; // v1: single page, no nav

  // BH-11.x: apply the resolved theme (system | light | dark) to <html>.
  useTheme((config?.general.theme as "system" | "light" | "dark" | undefined) ?? "system");

  useEffect(() => {
    void load();
  }, [load]);

  const SECTIONS: { id: Section; label: string }[] = [
    { id: "general",    label: t("settings-nav-general") },
    { id: "services",   label: t("settings-nav-services") },
    { id: "shortcut",   label: t("settings-nav-shortcut") },
    { id: "appearance", label: t("settings-nav-appearance") },
    { id: "about",      label: t("settings-nav-about") },
  ];

  if (loading && !config) {
    return <div className="p-8 text-fg-subtle">{t("common.loading", null, "Loading…")}</div>;
  }
  if (error) {
    return (
      <div className="p-8 text-red-500">
        {t("common.load-config-failed", { msg: error }, `Failed to load config: ${error}`)}
      </div>
    );
  }
  if (!config) return null;

  return (
    <div className="flex h-full">
      <aside className="w-48 border-r border-border bg-bg-subtle p-3">
        <h1 className="px-2 pb-2 text-sm font-semibold text-fg">{t("app-name", null, "translator")}</h1>
        <nav className="flex flex-col gap-0.5">
          {SECTIONS.map((s) => (
            <a
              key={s.id}
              href={`#${s.id}`}
              className="rounded-md px-2 py-1 text-sm text-fg-subtle hover:bg-border hover:text-fg"
            >
              {s.label}
            </a>
          ))}
        </nav>
      </aside>
      <main className="flex-1 overflow-y-auto p-6 space-y-6">
        <Section id="general" title={t("settings-nav-general")}>
          <GeneralSection />
        </Section>
        <Section id="services" title={t("settings-services-title")}>
          <ServicesSection />
        </Section>
        <Section id="shortcut" title={t("settings-nav-shortcut")}>
          <ShortcutSection />
        </Section>
        <Section id="appearance" title={t("settings-nav-appearance")}>
          <AppearanceSection />
        </Section>
        <Section id="about" title={t("settings-nav-about")}>
          <AboutSection />
        </Section>
        <pre className="text-xs text-fg-subtle">{JSON.stringify({ section }, null, 2)}</pre>
      </main>
    </div>
  );
}

function Section({ id, title, children }: { id: string; title: string; children: React.ReactNode }) {
  return (
    <section id={id} className="space-y-3">
      <h2 className="text-lg font-semibold">{title}</h2>
      <div className="card">{children}</div>
    </section>
  );
}
