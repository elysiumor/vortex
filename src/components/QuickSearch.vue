<script setup lang="ts">
import { ref, watch } from "vue";
import { Film, Tv, Home, Layers, Download, History, BarChart3, Copy, Settings, SunMoon, RefreshCw, Search } from "@lucide/vue";
import { api, posterSrc, type MediaItem } from "../lib/api";
import { CommandDialog, CommandEmpty, CommandGroup, CommandInput, CommandItem, CommandList, CommandSeparator } from "@/components/ui/command";

/**
 * Ctrl+K: jump to any title by name, go to a page, or run an action,
 * without leaving the keyboard.
 */
type Page = "home" | "movies" | "series" | "collections" | "downloads" | "history" | "stats" | "duplicates" | "settings";
const open = defineModel<boolean>("open", { required: true });
const emit = defineEmits<{ openItem: [id: number]; go: [page: Page]; toggleTheme: []; rescan: []; search: [query: string] }>();

const titles = ref<MediaItem[]>([]);
const typed = ref("");
watch(open, async (v) => {
  if (!v) return;
  typed.value = "";
  try { titles.value = await api.listMedia(); } catch { titles.value = []; }
});

const pages: { id: Page; label: string; icon: typeof Home }[] = [
  { id: "home", label: "Home", icon: Home },
  { id: "movies", label: "Movies", icon: Film },
  { id: "series", label: "Series", icon: Tv },
  { id: "collections", label: "Collections", icon: Layers },
  { id: "downloads", label: "Downloads", icon: Download },
  { id: "history", label: "History", icon: History },
  { id: "stats", label: "Statistics", icon: BarChart3 },
  { id: "duplicates", label: "Duplicates", icon: Copy },
  { id: "settings", label: "Settings", icon: Settings },
];

function run(fn: () => void) {
  open.value = false;
  fn();
}
</script>

<template>
  <CommandDialog v-model:open="open" title="Quick search" description="Jump to a title, a page or an action">
    <CommandInput placeholder="Search titles, pages and actions…" @input="(e: Event) => (typed = (e.target as HTMLInputElement).value)" />
    <CommandList class="max-h-[420px]">
      <CommandEmpty>Nothing matches “{{ typed }}”.</CommandEmpty>
      <CommandGroup v-if="titles.length" heading="Titles">
        <CommandItem v-for="m in titles" :key="m.id" :value="`${m.title} ${m.year ?? ''} ${m.kind}`" class="gap-3 py-1.5" @select="run(() => emit('openItem', m.id))">
          <img v-if="posterSrc(m)" :src="posterSrc(m)!" alt="" loading="lazy" decoding="async" class="h-10 w-7 shrink-0 rounded object-cover" />
          <div v-else class="grid h-10 w-7 shrink-0 place-items-center rounded bg-muted text-xs font-bold text-muted-foreground">{{ m.title.slice(0, 1) }}</div>
          <div class="min-w-0 flex-1">
            <div class="truncate font-medium">{{ m.title }}</div>
            <div class="truncate text-xs text-muted-foreground">{{ [m.year, m.kind === "movie" ? "Movie" : "Series", m.genres?.split(", ")[0]].filter(Boolean).join(" · ") }}</div>
          </div>
          <component :is="m.kind === 'movie' ? Film : Tv" class="text-muted-foreground" />
        </CommandItem>
      </CommandGroup>
      <CommandSeparator />
      <CommandGroup heading="Go to">
        <CommandItem v-for="p in pages" :key="p.id" :value="`go ${p.label}`" @select="run(() => emit('go', p.id))">
          <component :is="p.icon" /> {{ p.label }}
        </CommandItem>
      </CommandGroup>
      <CommandSeparator />
      <CommandGroup heading="Actions">
        <!-- Keyed by the text: the palette reads an item's text once, when it
             mounts, so without a remount this stopped matching after one letter. -->
        <CommandItem v-if="typed.trim()" :key="typed" :value="`search everywhere ${typed}`" @select="run(() => emit('search', typed))">
          <Search /> Search everything for “{{ typed }}”
        </CommandItem>
        <CommandItem value="toggle light dark theme" @select="run(() => emit('toggleTheme'))"><SunMoon /> Switch light / dark</CommandItem>
        <CommandItem value="rescan libraries scan" @select="run(() => emit('rescan'))"><RefreshCw /> Rescan libraries</CommandItem>
      </CommandGroup>
    </CommandList>
  </CommandDialog>
</template>
