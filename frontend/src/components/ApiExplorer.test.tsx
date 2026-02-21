import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { render, fireEvent, waitFor, cleanup } from "@solidjs/testing-library";
import ApiExplorer from "./ApiExplorer";

// Mock navigator.clipboard
const clipboardWriteText = vi.fn().mockResolvedValue(undefined);
Object.defineProperty(navigator, "clipboard", {
  value: { writeText: clipboardWriteText },
  writable: true,
  configurable: true,
});

// Mock fetch with a simple response
const mockFetch = vi.fn().mockResolvedValue({
  ok: true,
  text: () => Promise.resolve('{"ip":"1.2.3.4"}'),
});
global.fetch = mockFetch;

describe("ApiExplorer", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    cleanup();
  });

  afterEach(() => {
    cleanup();
  });

  it("renders collapsed by default", () => {
    const { queryByRole } = render(() => <ApiExplorer />);
    // The endpoint tabs should not be visible before expanding
    expect(queryByRole("tablist")).toBeNull();
  });

  it("expands when the header button is clicked", async () => {
    const { getByText, findByRole } = render(() => <ApiExplorer />);
    fireEvent.click(getByText("API Explorer"));
    const tablist = await findByRole("tablist");
    expect(tablist).toBeInTheDocument();
  });

  it("uses cached response on second visit to same endpoint+format", async () => {
    const { getByText, findByRole } = render(() => <ApiExplorer />);
    fireEvent.click(getByText("API Explorer"));
    await findByRole("tablist");

    // Wait for initial fetch for /|json to complete and be cached.
    // Must wait for the *response* pre (no class), not the loading pre.
    await waitFor(() =>
      expect(document.querySelector(".response-block pre:not(.response-loading)")).toBeTruthy()
    );
    expect(mockFetch).toHaveBeenCalledTimes(1);

    // Switch to yaml format — triggers a new fetch
    const yamlBtn = Array.from(
      document.querySelectorAll<HTMLButtonElement>(".format-pill")
    ).find((b) => b.textContent === "yaml")!;
    fireEvent.click(yamlBtn);
    await waitFor(() =>
      expect(document.querySelector(".response-block pre:not(.response-loading)")).toBeTruthy()
    );
    expect(mockFetch).toHaveBeenCalledTimes(2);

    // Switch back to json — should hit the cache, no new fetch
    const jsonBtn = Array.from(
      document.querySelectorAll<HTMLButtonElement>(".format-pill")
    ).find((b) => b.textContent === "json")!;
    fireEvent.click(jsonBtn);
    // Give it a moment, then confirm fetch count is still 2
    await new Promise((r) => setTimeout(r, 50));
    expect(mockFetch).toHaveBeenCalledTimes(2);
  });

  it("copy button toggles to copied state on click", async () => {
    const { getByText, findByLabelText } = render(() => <ApiExplorer />);
    fireEvent.click(getByText("API Explorer"));

    const copyBtn = await findByLabelText("Copy curl command");
    fireEvent.click(copyBtn);

    await waitFor(() =>
      expect(clipboardWriteText).toHaveBeenCalledTimes(1)
    );
    await waitFor(() =>
      expect(copyBtn).toHaveAttribute("aria-label", "Copied!")
    );
  });

  it("endpoint tabs support arrow-key navigation", async () => {
    const { getByText, findByRole } = render(() => <ApiExplorer />);
    fireEvent.click(getByText("API Explorer"));
    await findByRole("tablist");

    const tabs = Array.from(
      document.querySelectorAll<HTMLButtonElement>(".endpoint-tab")
    );
    expect(tabs.length).toBeGreaterThan(1);

    // Focus the first tab and press ArrowRight
    tabs[0].focus();
    fireEvent.keyDown(tabs[0], { key: "ArrowRight" });
    expect(document.activeElement).toBe(tabs[1]);

    // Press ArrowLeft to go back
    fireEvent.keyDown(tabs[1], { key: "ArrowLeft" });
    expect(document.activeElement).toBe(tabs[0]);
  });
});
