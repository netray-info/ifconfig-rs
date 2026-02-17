import { createSignal, onMount } from "solid-js";

export default function ThemeToggle() {
  const [theme, setTheme] = createSignal<"dark" | "light">("dark");

  onMount(() => {
    const saved = localStorage.getItem("theme");
    if (saved === "light" || saved === "dark") {
      setTheme(saved);
    } else if (window.matchMedia("(prefers-color-scheme: light)").matches) {
      setTheme("light");
    }
    document.documentElement.setAttribute("data-theme", theme());
  });

  const toggle = () => {
    const next = theme() === "dark" ? "light" : "dark";
    setTheme(next);
    localStorage.setItem("theme", next);
    document.documentElement.setAttribute("data-theme", next);
  };

  return (
    <button class="theme-toggle" onClick={toggle} title="Toggle theme">
      {theme() === "dark" ? "\u2600" : "\u263E"}
    </button>
  );
}
