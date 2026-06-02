import { create } from "zustand";
import type { Config } from "../types/bindings";
import * as api from "../ipc/commands";

interface ConfigState {
  config: Config | null;
  loading: boolean;
  error: string | null;
  load: () => Promise<void>;
  save: (cfg: Config) => Promise<void>;
}

export const useConfigStore = create<ConfigState>((set) => ({
  config: null,
  loading: false,
  error: null,
  load: async () => {
    set({ loading: true, error: null });
    try {
      const cfg = await api.getConfig();
      set({ config: cfg, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },
  save: async (cfg) => {
    set({ error: null });
    try {
      await api.saveConfig(cfg);
      set({ config: cfg });
    } catch (e) {
      set({ error: String(e) });
    }
  },
}));
