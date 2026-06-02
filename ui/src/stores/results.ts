import { create } from "zustand";
import type { ServiceOutcomeDto } from "../types/bindings";

interface ResultsState {
  outcomes: ServiceOutcomeDto[];
  loading: boolean;
  error: string | null;
  queryText: string | null;
  setOutcomes: (outcomes: ServiceOutcomeDto[]) => void;
  setLoading: (loading: boolean) => void;
  setError: (error: string | null) => void;
  setQueryText: (text: string | null) => void;
  reset: () => void;
}

export const useResultsStore = create<ResultsState>((set) => ({
  outcomes: [],
  loading: false,
  error: null,
  queryText: null,
  setOutcomes: (outcomes) => set({ outcomes, loading: false, error: null }),
  setLoading: (loading) => set({ loading, error: null }),
  setError: (error) => set({ error, loading: false }),
  setQueryText: (queryText) => set({ queryText }),
  reset: () => set({ outcomes: [], loading: false, error: null, queryText: null }),
}));
