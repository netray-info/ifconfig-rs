import { createSignal, onMount, onCleanup } from "solid-js";

type ThemeChoice = "dark" | "light" | "system";

function resolveSystemTheme(): "dark" | "light" {
  return window.matchMedia("(prefers-color-scheme: light)").matches ? "light" : "dark";
}

function applyTheme(choice: ThemeChoice) {
  const resolved = choice === "system" ? resolveSystemTheme() : choice;
  document.documentElement.setAttribute("data-theme", resolved);
}

export default function ThemeToggle() {
  const [theme, setTheme] = createSignal<ThemeChoice>("system");

  const onSystemChange = () => {
    if (theme() === "system") {
      applyTheme("system");
    }
  };

  onMount(() => {
    const saved = localStorage.getItem("theme");
    if (saved === "light" || saved === "dark" || saved === "system") {
      setTheme(saved);
    }
    // Theme is already applied synchronously via inline script in index.html

    window.matchMedia("(prefers-color-scheme: light)").addEventListener("change", onSystemChange);
  });

  onCleanup(() => {
    window.matchMedia("(prefers-color-scheme: light)").removeEventListener("change", onSystemChange);
  });

  const toggle = () => {
    const order: ThemeChoice[] = ["dark", "light", "system"];
    const next = order[(order.indexOf(theme()) + 1) % order.length];
    setTheme(next);
    localStorage.setItem("theme", next);
    applyTheme(next);
  };

  const icon = () => {
    switch (theme()) {
      case "dark": return "\u263E";
      case "light": return "\u2600";
      case "system": return "\u25D1";
    }
  };

  const label = () => {
    switch (theme()) {
      case "dark": return "Dark";
      case "light": return "Light";
      case "system": return "System";
    }
  };

  return (
    <button class="theme-toggle" onClick={toggle} title={`Theme: ${label()}. Click to switch.`} aria-label={`Theme: ${label()}. Click to switch.`}>
      {icon()}
    </button>
  );
}
