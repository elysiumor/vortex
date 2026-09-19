<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { Toaster, toast } from "vue-sonner";
import { Home, Film, Tv, Layers, History, Copy, BarChart3, Settings, Search, Sparkles, X, Play, Trash2 } from "@lucide/vue";
import { api, type SmartList } from "./lib/api";
import { hms } from "./lib/format";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { TooltipProvider } from "@/components/ui/tooltip";
import HomeView from "./components/HomeView.vue";
import LibraryView from "./components/LibraryView.vue";
import SeriesView from "./components/SeriesView.vue";
import SettingsView from "./components/SettingsView.vue";
import SearchView from "./components/SearchView.vue";
import HistoryView from "./components/HistoryView.vue";
import DuplicatesView from "./components/DuplicatesView.vue";
import CollectionsView from "./components/CollectionsView.vue";
import StatsView from "./components/StatsView.vue";

type Page = "home" | "movies" | "series" | "collections" | "history" | "duplicates" | "stats" | "settings" | "smart";
const page = ref<Page>("home");
const openId = ref<number | null>(null);
const cameFrom = ref<Page>("home");
const search = ref("");
const searchBox = ref<InstanceType<typeof Input>>();
const dupCount = ref(0);
const smartLists = ref<SmartList[]>([]);
const activeSmart = ref<SmartList | null>(null);

const home = ref<InstanceType<typeof HomeView>>();
const library = ref<InstanceType<typeof LibraryView>>();
const detail = ref<InstanceType<typeof SeriesView>>();
const history = ref<InstanceType<typeof HistoryView>>();
const duplicates = ref<InstanceType<typeof DuplicatesView>>();
const collectionsRef = ref<InstanceType<typeof CollectionsView>>();
const statsRef = ref<InstanceType<typeof StatsView>>();

const nav = computed(() => [
  { id: "home" as Page, label: "Home", icon: Home },
  { id: "movies" as Page, label: "Movies", icon: Film },
  { id: "series" as Page, label: "TV Series", icon: Tv },
  { id: "collections" as Page, label: "Collections", icon: Layers },
  { id: "history" as Page, label: "History", icon: History },
  { id: "stats" as Page, label: "Statistics", icon: BarChart3 },
  { id: "duplicates" as Page, label: "Duplicates", icon: Copy, count: dupCount.value },
]);

function go(p: Page, smart: SmartList | null = null) {
  page.value = p;
  activeSmart.value = smart;
  openId.value = null;
  search.value = "";
}

function openItem(id: number) {
  cameFrom.value = page.value;
  openId.value = id;
  search.value = "";
}

function back() {
  openId.value = null;
  page.value = cameFrom.value;
}

async function refreshDupCount() {
  dupCount.value = (await api.findDuplicates()).length;
}

async function loadSmartLists() {
  const s = await api.getSettings();
  try {
    smartLists.value = s.smart_lists ? (JSON.parse(s.smart_lists) as SmartList[]) : [];
  } catch {
    smartLists.value = [];
  }
}

async function saveSmartLists(lists: SmartList[]) {
  smartLists.value = lists;
  await api.setSetting("smart_lists", JSON.stringify(lists));
}

async function addSmartList(l: SmartList) {
  await saveSmartLists([...smartLists.value, l]);
  toast.success(`Smart list "${l.name}" saved`);
}

async function removeSmartList(l: SmartList) {
  await saveSmartLists(smartLists.value.filter((x) => x.id !== l.id));
  if (activeSmart.value?.id === l.id) go("home");
}

function refreshAll() {
  home.value?.reload();
  library.value?.reload();
  detail.value?.reload();
  history.value?.reload();
  duplicates.value?.reload();
  collectionsRef.value?.reload();
  statsRef.value?.reload();
  refreshDupCount();
}

async function onScanned() {
  refreshAll();
  const s = await api.getSettings();
  if (s.tmdb_key) api.fetchPosters(false).catch(() => {});
  api.probeDurations().catch(() => {});
}

// ---- "Up next" countdown after an episode finishes with the player closed ----
const upNext = ref<{ id: number; label: string; seconds: number } | null>(null);
let upNextTimer: number | undefined;

function startUpNext(id: number, label: string) {
  cancelUpNext();
  upNext.value = { id, label, seconds: 10 };
  upNextTimer = window.setInterval(() => {
    if (!upNext.value) return;
    upNext.value.seconds -= 1;
    if (upNext.value.seconds <= 0) playUpNext();
  }, 1000);
}
function cancelUpNext() {
  clearInterval(upNextTimer);
  upNext.value = null;
}
async function playUpNext() {
  const target = upNext.value;
  cancelUpNext();
  if (!target) return;
  try {
    await api.playEpisode(target.id);
  } catch (e) {
    toast.error(String(e));
  }
}

// ---- keyboard ----
function onKey(e: KeyboardEvent) {
  const el = (searchBox.value as unknown as { $el?: HTMLInputElement })?.$el;
  const inField = ["INPUT", "TEXTAREA", "SELECT"].includes((e.target as HTMLElement)?.tagName);
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
    e.preventDefault();
    el?.focus();
    el?.select();
  } else if (e.key === "/" && !inField) {
    e.preventDefault();
    el?.focus();
  } else if (e.key === "Escape") {
    if (document.activeElement === el) {
      search.value = "";
      el?.blur();
    } else if (!inField && openId.value !== null) {
      back();
    }
  }
}

const unlisteners: (() => void)[] = [];
onMounted(async () => {
  window.addEventListener("keydown", onKey);
  unlisteners.push(() => window.removeEventListener("keydown", onKey));
  refreshDupCount();
  loadSmartLists();
  unlisteners.push(await api.onPlaybackEnded((e) => {
    const how = e.exact ? "" : " (estimated)";
    if (e.auto_started && e.next_label) toast.success(`Playing next: ${e.next_label}`);
    else if (e.completed && e.next_episode_id && e.next_label) startUpNext(e.next_episode_id, e.next_label);
    else if (e.completed) toast.success("Marked as watched");
    else toast(`Progress saved at ${hms(e.position_secs)}${how}`);
    refreshAll();
  }));
  unlisteners.push(await api.onScanDone((p) => {
    const s = p.stats;
    if (s.added || s.removed || s.renamed) {
      const parts: string[] = [];
      if (s.added) parts.push(`${s.added} added`);
      if (s.renamed) parts.push(`${s.renamed} renamed`);
      if (s.removed) parts.push(`${s.removed} removed`);
      if (s.extras) parts.push(`${s.extras} extras`);
      const why = p.reason === "watch" ? "Folder change" : p.reason === "drive" ? "Drive connected" : "Rescan";
      toast(`${why}: ${parts.join(", ")}`);
    }
    refreshAll();
  }));
  unlisteners.push(await api.onLibraryRestored(() => { openId.value = null; refreshAll(); }));
  unlisteners.push(await api.onDurationsDone((p) => { if (p.found > 0) refreshAll(); }));
  unlisteners.push(await api.onPostersDone(() => refreshAll()));
});
onUnmounted(() => unlisteners.forEach((u) => u()));
</script>

<template>
  <TooltipProvider :delay-duration="300">
    <div class="flex h-full">
      <aside class="flex w-56 shrink-0 flex-col gap-1 border-r border-sidebar-border bg-sidebar p-3">
        <div class="flex items-center gap-2 px-2 pb-3 pt-1">
          <div class="grid size-7 place-items-center rounded-lg bg-primary text-primary-foreground"><Play class="size-4 fill-current" /></div>
          <span class="text-base font-semibold tracking-tight">Vortex</span>
        </div>

        <div class="relative mb-2">
          <Search class="pointer-events-none absolute left-2.5 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
          <Input ref="searchBox" v-model="search" placeholder="Search" class="h-9 pl-8 pr-12 bg-background/60" />
          <kbd class="pointer-events-none absolute right-2 top-1/2 -translate-y-1/2 rounded border bg-muted px-1.5 text-[10px] text-muted-foreground">Ctrl K</kbd>
        </div>

        <button v-for="n in nav" :key="n.id"
                class="flex h-9 items-center gap-2.5 rounded-md px-2.5 text-sm transition-colors"
                :class="page === n.id && !openId && !search ? 'bg-primary/12 text-primary font-medium' : 'text-sidebar-foreground hover:bg-accent'"
                @click="go(n.id)">
          <component :is="n.icon" class="size-4" />
          <span class="flex-1 text-left">{{ n.label }}</span>
          <span v-if="n.count" class="rounded-full bg-muted px-1.5 text-[11px] text-muted-foreground">{{ n.count }}</span>
        </button>

        <template v-if="smartLists.length">
          <div class="mt-3 px-2.5 text-[11px] font-medium uppercase tracking-wider text-muted-foreground">Smart lists</div>
          <div v-for="l in smartLists" :key="l.id" class="group flex items-center">
            <button class="flex h-8 flex-1 items-center gap-2.5 rounded-md px-2.5 text-sm transition-colors"
                    :class="activeSmart?.id === l.id ? 'bg-primary/12 text-primary font-medium' : 'text-sidebar-foreground hover:bg-accent'"
                    @click="go('smart', l)">
              <Sparkles class="size-3.5" /><span class="flex-1 truncate text-left">{{ l.name }}</span>
            </button>
            <Button variant="ghost" size="icon-xs" class="opacity-0 group-hover:opacity-100 text-muted-foreground" title="Delete smart list" @click="removeSmartList(l)"><Trash2 /></Button>
          </div>
        </template>

        <div class="flex-1"></div>
        <button class="flex h-9 items-center gap-2.5 rounded-md px-2.5 text-sm transition-colors"
                :class="page === 'settings' ? 'bg-primary/12 text-primary font-medium' : 'text-sidebar-foreground hover:bg-accent'"
                @click="go('settings')">
          <Settings class="size-4" /><span>Settings</span>
        </button>
      </aside>

      <main class="flex-1 overflow-y-auto">
        <div class="mx-auto max-w-[1600px] p-6">
          <SearchView v-if="search.trim()" :query="search" @open="openItem" />
          <SeriesView v-else-if="openId !== null" ref="detail" :id="openId" @back="back" />
          <HomeView v-else-if="page === 'home'" ref="home" @open="openItem" @go="go" />
          <LibraryView v-else-if="page === 'movies'" ref="library" kind="movie" @open="openItem" @save-smart="addSmartList" />
          <LibraryView v-else-if="page === 'series'" ref="library" kind="series" @open="openItem" @save-smart="addSmartList" />
          <LibraryView v-else-if="page === 'smart' && activeSmart" ref="library" :kind="activeSmart.kind === 'all' ? undefined : activeSmart.kind" :smart="activeSmart" @open="openItem" @save-smart="addSmartList" />
          <CollectionsView v-else-if="page === 'collections'" ref="collectionsRef" @open="openItem" />
          <HistoryView v-else-if="page === 'history'" ref="history" @open="openItem" />
          <StatsView v-else-if="page === 'stats'" ref="statsRef" @open="openItem" />
          <DuplicatesView v-else-if="page === 'duplicates'" ref="duplicates" @open="openItem" />
          <SettingsView v-else @scanned="onScanned" />
        </div>
      </main>

      <Toaster position="bottom-right" :theme="'system'" rich-colors close-button />

      <div v-if="upNext" class="fixed bottom-5 right-5 z-50 flex items-center gap-3 rounded-xl border border-primary/40 bg-popover p-3 pl-4 shadow-2xl animate-in slide-in-from-bottom-4">
        <div class="max-w-xs">
          <div class="text-xs text-muted-foreground">Up next in {{ upNext.seconds }}s</div>
          <div class="truncate font-medium">{{ upNext.label }}</div>
        </div>
        <Button size="sm" @click="playUpNext"><Play class="fill-current" /> Play now</Button>
        <Button size="icon-sm" variant="ghost" @click="cancelUpNext"><X /></Button>
      </div>
    </div>
  </TooltipProvider>
</template>
