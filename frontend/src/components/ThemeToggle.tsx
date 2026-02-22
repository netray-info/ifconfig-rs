import { createSignal, onMount } from "solid-js";

type ThemeChoice = "dark" | "light" | "system";

function applyTheme(choice: ThemeChoice) {
  if (choice === "system") {
    // Remove the attribute so the CSS @media (prefers-color-scheme) rule takes over.
    document.documentElement.removeAttribute("data-theme");
  } else {
    document.documentElement.setAttribute("data-theme", choice);
  }
}

export default function ThemeToggle() {
  const [theme, setTheme] = createSignal<ThemeChoice>("system");

  onMount(() => {
    const saved = localStorage.getItem("theme");
    const choice: ThemeChoice =
      saved === "light" || saved === "dark" || saved === "system" ? saved : "system";
    setTheme(choice);
    applyTheme(choice);
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
