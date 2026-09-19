import { onMounted, onUnmounted, type Ref } from "vue";

/**
 * Arrow-key navigation across focusable cards (`[data-card]`) laid out in a
 * grid. Enter activates the card; P dispatches a "play" event on it.
 */
export function useGridKeys(container: Ref<HTMLElement | undefined>) {
  function cards(): HTMLElement[] {
    return Array.from(container.value?.querySelectorAll<HTMLElement>("[data-card]") ?? []);
  }

  function onKey(e: KeyboardEvent) {
    const target = e.target as HTMLElement | null;
    if (!target || target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.tagName === "SELECT") return;
    if (!target.hasAttribute("data-card")) return;
    const list = cards();
    const i = list.indexOf(target);
    if (i < 0) return;

    const rect = target.getBoundingClientRect();
    const sameRow = (el: HTMLElement) => Math.abs(el.getBoundingClientRect().top - rect.top) < rect.height / 2;
    let next: HTMLElement | undefined;

    switch (e.key) {
      case "ArrowRight": next = list[i + 1]; break;
      case "ArrowLeft": next = list[i - 1]; break;
      case "ArrowDown": {
        next = list.slice(i + 1).find((el) => !sameRow(el) && Math.abs(el.getBoundingClientRect().left - rect.left) < rect.width / 2)
          ?? list.slice(i + 1).find((el) => !sameRow(el));
        break;
      }
      case "ArrowUp": {
        const before = list.slice(0, i).reverse();
        next = before.find((el) => !sameRow(el) && Math.abs(el.getBoundingClientRect().left - rect.left) < rect.width / 2)
          ?? before.find((el) => !sameRow(el));
        break;
      }
      case "Home": next = list[0]; break;
      case "End": next = list[list.length - 1]; break;
      case "Enter": target.click(); e.preventDefault(); return;
      case "p": case "P": target.dispatchEvent(new CustomEvent("play", { bubbles: false })); e.preventDefault(); return;
      default: return;
    }
    if (next) {
      e.preventDefault();
      next.focus();
      next.scrollIntoView({ block: "nearest" });
    }
  }

  onMounted(() => container.value?.addEventListener("keydown", onKey));
  onUnmounted(() => container.value?.removeEventListener("keydown", onKey));
}
