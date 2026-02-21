import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, fireEvent, cleanup } from "@solidjs/testing-library";
import ThemeToggle from "./ThemeToggle";

describe("ThemeToggle", () => {
  beforeEach(() => {
    localStorage.clear();
    document.documentElement.removeAttribute("data-theme");
    cleanup();
  });

  it("reads saved theme from localStorage on mount", () => {
    localStorage.setItem("theme", "light");
    const { getByRole } = render(() => <ThemeToggle />);
    const btn = getByRole("button");
    expect(btn).toHaveAttribute("aria-label", expect.stringContaining("Light"));
  });

  it("cycles dark → light → system on successive clicks", () => {
    localStorage.setItem("theme", "dark");
    const { getByRole } = render(() => <ThemeToggle />);
    const btn = getByRole("button");

    // dark → light
    fireEvent.click(btn);
    expect(localStorage.getItem("theme")).toBe("light");
    expect(btn).toHaveAttribute("aria-label", expect.stringContaining("Light"));

    // light → system
    fireEvent.click(btn);
    expect(localStorage.getItem("theme")).toBe("system");
    expect(btn).toHaveAttribute("aria-label", expect.stringContaining("System"));
  });

  it("persists the new theme to localStorage on click", () => {
    const { getByRole } = render(() => <ThemeToggle />);
    const btn = getByRole("button");
    fireEvent.click(btn);
    const saved = localStorage.getItem("theme");
    expect(["dark", "light", "system"]).toContain(saved);
  });

  it("applies data-theme attribute to documentElement on click", () => {
    localStorage.setItem("theme", "dark");
    const { getByRole } = render(() => <ThemeToggle />);
    fireEvent.click(getByRole("button")); // dark → light
    expect(document.documentElement.getAttribute("data-theme")).toBe("light");
  });
});
