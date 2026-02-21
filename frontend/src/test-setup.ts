import "@testing-library/jest-dom";
import { vi } from "vitest";

// happy-dom v20 does not expose a working localStorage in the test environment.
// Provide a Map-backed mock that covers the full Storage interface.
const store = new Map<string, string>();
vi.stubGlobal("localStorage", {
  getItem: (k: string) => store.get(k) ?? null,
  setItem: (k: string, v: string) => { store.set(k, v); },
  removeItem: (k: string) => { store.delete(k); },
  clear: () => { store.clear(); },
  get length() { return store.size; },
  key: (i: number) => [...store.keys()][i] ?? null,
});
