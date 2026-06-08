import { create } from "zustand";
import type { ServiceOutcomeDto } from "../types/bindings";

interface ResultsState {
  outcomes: ServiceOutcomeDto[];
  loading: boolean;
  error: string | null;
  queryText: string | null;
  setOutcomes: (outcomes: ServiceOutcomeDto[]) => void;
  setPendingOutcomes: (outcomes: ServiceOutcomeDto[]) => void;
  mergeOutcome: (outcome: ServiceOutcomeDto) => void;
  setLoading: (loading: boolean) => void;
  setError: (error: string | null) => void;
  setQueryText: (text: string | null) => void;
  finishLoading: () => void;
  reset: () => void;
}

export const useResultsStore = create<ResultsState>((set) => ({
  outcomes: [],
  loading: false,
  error: null,
  queryText: null,
  setOutcomes: (outcomes) => set({ outcomes, loading: false, error: null }),
  setPendingOutcomes: (outcomes) =>
    set({ outcomes, loading: outcomes.length > 0, error: null }),
  mergeOutcome: (outcome) =>
    set((state) => {
      const index = state.outcomes.findIndex(
        (item) => item.service_id === outcome.service_id,
      );
      if (index === -1) {
        return { outcomes: [...state.outcomes, outcome], error: null };
      }
      const outcomes = [...state.outcomes];
      outcomes[index] = outcome;
      return { outcomes, error: null };
    }),
  setLoading: (loading) => set({ loading, error: null }),
  setError: (error) => set({ error, loading: false }),
  setQueryText: (queryText) => set({ queryText }),
  finishLoading: () => set({ loading: false }),
  reset: () =>
    set({ outcomes: [], loading: false, error: null, queryText: null }),
}));
