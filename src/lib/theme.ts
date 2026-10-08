import { ref } from "vue";

export type Theme = "system" | "light" | "dark";

const KEY = "vortex-theme";
const media = window.matchMedia("(prefers-color-scheme: dark)");

function render(t: Theme) {
  const dark = t === "dark" || (t === "system" && media.matches);
  document.documentElement.classList.toggle("dark", dark);
  document.documentElement.style.colorScheme = dark ? "dark" : "light";
}

/** The chosen theme, shared by every place that can change it. */
export const currentTheme = ref<Theme>("system");

export function applyTheme(t: Theme) {
  currentTheme.value = t;
  render(t);
  try {
    localStorage.setItem(KEY, t);
  } catch {}
}

export function loadTheme(): Theme {
  try {
    const v = localStorage.getItem(KEY);
    if (v === "light" || v === "dark" || v === "system") return v;
  } catch {}
  return "system";
}

/** Call once at startup so the first paint already has the right colours. */
export function initTheme(): Theme {
  const t = loadTheme();
  currentTheme.value = t;
  render(t);
  media.addEventListener("change", () => render(loadTheme()));
  return t;
}
