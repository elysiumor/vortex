<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { Toaster, toast } from "vue-sonner";
import { History, Copy, BarChart3, Settings, Search, Sparkles, X, Play, Trash2, ChevronDown, Sun, Moon, RefreshCw, Loader2, Clapperboard } from "@lucide/vue";
import { api, type JobProgress, type SmartList } from "./lib/api";
import { hms } from "./lib/format";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { TooltipProvider } from "@/components/ui/tooltip";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuLabel, DropdownMenuSeparator, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { applyTheme } from "./lib/theme";
import HomeView from "./components/HomeView.vue";
import LibraryView from "./components/LibraryView.vue";
import SeriesView from "./components/SeriesView.vue";
import SettingsView from "./components/SettingsView.vue";
import SearchView from "./components/SearchView.vue";
import HistoryView from "./components/HistoryView.vue";
import DuplicatesView from "./components/DuplicatesView.vue";
import CollectionsView from "./components/CollectionsView.vue";
import StatsView from "./components/StatsView.vue";
import DownloadsView from "./components/DownloadsView.vue";
import TmdbView from "./components/TmdbView.vue";
import QuickSearch from "./components/QuickSearch.vue";

type Page = "home" | "movies" | "series" | "collections" | "history" | "duplicates" | "stats" | "downloads" | "tmdb" | "settings" | "smart";
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
const downloadsRef = ref<InstanceType<typeof DownloadsView>>();

const nav: { id: Page; label: string }[] = [
  { id: "home", label: "Home" },
  { id: "movies", label: "Movies" },
  { id: "series", label: "Series" },
  { id: "collections", label: "Collections" },
  { id: "downloads", label: "Downloads" },
];
const more = computed(() => [
  { id: "history" as Page, label: "History", icon: History },
  { id: "stats" as Page, label: "Statistics", icon: BarChart3 },
  { id: "duplicates" as Page, label: "Duplicates", icon: Copy, count: dupCount.value },
  { id: "tmdb" as Page, label: "TMDb", icon: Clapperboard },
]);
const inMore = computed(() => !openId.value && !search.value && (more.value.some((m) => m.id === page.value) || page.value === "smart"));

// Pages that open on a full-width picture slide under a transparent bar;
// the bar turns frosted once the page scrolls.
const scroller = ref<HTMLElement>();
const scrolled = ref(false);
const heroPage = computed(() => !search.value.trim() && (openId.value !== null || page.value === "home"));
function onScroll() { scrolled.value = (scroller.value?.scrollTop ?? 0) > 24; }
watch([page, openId, activeSmart], () => { scroller.value?.scrollTo({ top: 0 }); scrolled.value = false; });

// ---- background work: the status pill and the Sync button ----
// One entry per job while it runs; the backend's last event for a job has
// `running` false and drops it here.
const jobs = ref<Partial<Record<JobProgress["job"], JobProgress>>>({});
// Shown in the order a person would want to know about them: a rename
// touching their files before a poster fetch that can run for an hour.
const JOB_ORDER: JobProgress["job"][] = ["rename", "scan", "memory", "posters", "durations"];
const activeJob = computed(() => JOB_ORDER.map((j) => jobs.value[j]).find((j) => j?.running) ?? null);
const otherJobs = computed(() => JOB_ORDER.map((j) => jobs.value[j]).filter((j) => j?.running && j !== activeJob.value) as JobProgress[]);
const jobPct = computed(() => {
  const j = activeJob.value;
  return j && j.total > 0 ? Math.min(100, Math.round((100 * j.done) / j.total)) : 0;
});
const jobTitle = computed(() => {
  const j = activeJob.value;
  if (!j) return "";
  const where = j.detail ? ` · ${j.detail}` : "";
  const count = j.total > 0 ? ` · ${j.done} / ${j.total}` : "";
  return `${j.label}${where}${count}`;
});
function onJob(p: JobProgress) {
  const next = { ...jobs.value };
  if (p.running) next[p.job] = p; else delete next[p.job];
  jobs.value = next;
}

const syncing = ref(false);
/** Rescan every library, then refresh what is on screen. */
async function syncLibrary() {
  if (syncing.value) return;
  if (jobs.value.scan?.running) {
    toast("Already syncing; the library refreshes when it finishes.");
    return;
  }
  syncing.value = true;
  try {
    const s = await api.scanLibraries();
    const parts = [`${s.files_seen} files`];
    if (s.added) parts.push(`${s.added} added`);
    if (s.renamed) parts.push(`${s.renamed} renamed`);
    if (s.removed) parts.push(`${s.removed} removed`);
    if (s.libraries_skipped.length) parts.push(`${s.libraries_skipped.length} offline`);
    toast.success(`Synced: ${parts.join(", ")}`);
  } catch (e) {
    toast.error(String(e));
  } finally {
    syncing.value = false;
  }
  refreshAll();
}

// ---- Ctrl+K palette ----
const quickOpen = ref(false);
const rescanFromPalette = syncLibrary;

const dark = ref(document.documentElement.classList.contains("dark"));
function toggleTheme() {
  dark.value = !dark.value;
  applyTheme(dark.value ? "dark" : "light");
}
// Settings can change the theme too; follow it.
new MutationObserver(() => { dark.value = document.documentElement.classList.contains("dark"); })
  .observe(document.documentElement, { attributes: true, attributeFilter: ["class"] });

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

/**
 * Scan, poster and duration events often land within milliseconds of each
 * other, and reloading every view once per event means the same queries run
 * several times over. Collapse a burst into a single refresh; the delay is
 * short enough to be invisible.
 */
let refreshTimer: number | undefined;
function refreshAll() {
  clearTimeout(refreshTimer);
  refreshTimer = window.setTimeout(reloadViews, 120);
}

function reloadViews() {
  home.value?.reload();
  library.value?.reload();
  detail.value?.reload();
  history.value?.reload();
  duplicates.value?.reload();
  collectionsRef.value?.reload();
  statsRef.value?.reload();
  refreshDupCount();
}

// Posters and durations for new files are started by the backend when a scan
// changes something; asking again from here only collided with a running job.
function onScanned() {
  refreshAll();
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
    quickOpen.value = !quickOpen.value;
  } else if (e.key === "/" && !inField) {
    e.preventDefault();
    el?.focus();
  } else if (e.key === "Escape") {
    // A dialog or menu closes itself on Escape; don't also leave the page.
    if ((e.target as HTMLElement)?.closest?.('[role="dialog"],[role="alertdialog"],[role="menu"],[role="listbox"]')) return;
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
    // Settings reports its own scans.
    if (p.reason !== "manual" && (s.added || s.removed || s.renamed)) {
      const parts: string[] = [];
      if (s.added) parts.push(`${s.added} added`);
      if (s.renamed) parts.push(`${s.renamed} renamed`);
      if (s.removed) parts.push(`${s.removed} removed`);
      if (s.extras) parts.push(`${s.extras} extras`);
      const why = p.reason === "watch" ? "Folder change" : p.reason === "drive" ? "Drive connected" : p.reason === "torrent" ? "Download finished" : "Rescan";
      toast(`${why}: ${parts.join(", ")}`);
    }
    refreshAll();
  }));
  unlisteners.push(await api.onJobProgress(onJob));
  unlisteners.push(await api.onLibraryRestored(() => { openId.value = null; refreshAll(); }));
  unlisteners.push(await api.onLibraryChanged(() => refreshAll()));
  unlisteners.push(await api.onDurationsDone((p) => { if (p.found > 0) refreshAll(); }));
  unlisteners.push(await api.onPostersDone(() => refreshAll()));
  // magnet: links clicked in a browser: jump to Downloads and stream.
  const openMagnet = async (m: string) => {
    go("downloads");
    await nextTick();
    await downloadsRef.value?.streamFrom(m);
  };
  unlisteners.push(await api.onOpenUrl(openMagnet));
  unlisteners.push(await api.onTorrentDone((p) => {
    toast.success(p.moved.length ? `Download finished: ${p.name}` : `Download finished: ${p.name} (files stayed in .incomplete)`);
  }));
  // Last: a stream can buffer for a minute and may fail, and nothing above
  // should wait on it.
  for (const m of await api.pendingOpenUrls()) await openMagnet(m);
});
onUnmounted(() => {
  clearTimeout(refreshTimer);
  unlisteners.forEach((u) => u());
});
</script>

<template>
  <TooltipProvider :delay-duration="300">
    <div class="relative flex h-full flex-col">
      <!-- Top bar: transparent over a hero picture, frosted once the page scrolls. -->
      <header class="absolute left-0 right-[10px] top-0 z-40 transition-[background-color,box-shadow,backdrop-filter] duration-300"
              :class="scrolled || !heroPage ? 'glass shadow-[0_1px_0_var(--border)]' : 'bg-gradient-to-b from-background/80 via-background/30 to-transparent'">
        <div class="mx-auto flex h-16 max-w-[1920px] items-center gap-4 px-6 xl:gap-8 xl:px-10">
          <button class="group flex items-center gap-2.5" title="Home" @click="go('home')">
            <span class="grid size-8 place-items-center rounded-[10px] bg-brand shadow-[0_6px_18px_-6px_rgb(179_234_63/0.8)] transition-transform group-hover:rotate-[-8deg]">
              <Play class="ml-0.5 size-4 fill-current" />
            </span>
            <span class="text-brand text-[1.35rem] font-black tracking-[-0.04em]">VORTEX</span>
          </button>

          <nav class="flex items-center gap-1">
            <button v-for="n in nav" :key="n.id"
                    class="relative rounded-full px-3 py-1.5 text-sm font-medium transition-colors xl:px-3.5"
                    :class="page === n.id && !openId && !search ? 'text-foreground' : 'text-foreground/60 hover:text-foreground'"
                    @click="go(n.id)">
              {{ n.label }}
              <span v-if="page === n.id && !openId && !search" class="absolute inset-x-3.5 -bottom-[3px] h-[3px] rounded-full bg-brand"></span>
            </button>
            <DropdownMenu>
              <DropdownMenuTrigger as-child>
                <button class="relative flex items-center gap-1 rounded-full px-3 py-1.5 text-sm font-medium transition-colors xl:px-3.5"
                        :class="inMore ? 'text-foreground' : 'text-foreground/60 hover:text-foreground'">
                  More <ChevronDown class="size-3.5" />
                  <span v-if="dupCount" class="ml-0.5 rounded-full bg-primary/15 px-1.5 text-[10px] font-semibold text-primary">{{ dupCount }}</span>
                  <span v-if="inMore" class="absolute inset-x-3.5 -bottom-[3px] h-[3px] rounded-full bg-brand"></span>
                </button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="start" class="min-w-52">
                <DropdownMenuItem v-for="m in more" :key="m.id" @select="go(m.id)">
                  <component :is="m.icon" /> <span class="flex-1">{{ m.label }}</span>
                  <span v-if="m.count" class="rounded-full bg-muted px-1.5 text-[11px] text-muted-foreground">{{ m.count }}</span>
                </DropdownMenuItem>
                <template v-if="smartLists.length">
                  <DropdownMenuSeparator />
                  <DropdownMenuLabel class="text-xs text-muted-foreground">Smart lists</DropdownMenuLabel>
                  <DropdownMenuItem v-for="l in smartLists" :key="l.id" class="group" @select="go('smart', l)">
                    <Sparkles /> <span class="flex-1 truncate">{{ l.name }}</span>
                    <button class="rounded p-0.5 opacity-0 hover:text-destructive group-hover:opacity-100" title="Delete smart list" @click.stop="removeSmartList(l)"><Trash2 class="size-3.5" /></button>
                  </DropdownMenuItem>
                </template>
              </DropdownMenuContent>
            </DropdownMenu>
          </nav>

          <div class="flex-1"></div>

          <div class="relative min-w-0 shrink">
            <Search class="pointer-events-none absolute left-3.5 top-1/2 size-4 -translate-y-1/2 text-foreground/50" />
            <Input ref="searchBox" v-model="search" placeholder="Titles, people, episodes"
                   class="h-10 w-48 max-w-full rounded-full border-foreground/10 bg-foreground/[0.06] pl-10 pr-14 shadow-none backdrop-blur transition-[width,background-color] duration-300 placeholder:text-foreground/45 focus-visible:w-60 focus-visible:bg-background/80 xl:w-64 xl:focus-visible:w-80" />
            <button class="absolute right-2 top-1/2 -translate-y-1/2 rounded-md border border-foreground/10 px-1.5 text-[10px] text-foreground/45 hover:text-foreground" title="Quick search" @click="quickOpen = true">Ctrl K</button>
          </div>
          <div class="flex items-center gap-1">
            <!-- What is going on behind the scenes, while anything is. -->
            <div v-if="activeJob" class="glass relative mr-1 hidden min-w-0 max-w-[340px] items-center gap-2 overflow-hidden rounded-full border border-foreground/10 py-1.5 pl-3 pr-1.5 text-xs lg:flex"
                 :title="[jobTitle, ...otherJobs.map((j) => j.label)].join('\n')">
              <Loader2 class="size-3.5 shrink-0 animate-spin text-primary" />
              <span class="min-w-0 truncate"><span class="font-semibold">{{ activeJob.label }}</span><span v-if="activeJob.detail" class="text-foreground/60"> · {{ activeJob.detail }}</span></span>
              <span class="shrink-0 tabular-nums font-semibold" v-if="activeJob.total > 0">{{ jobPct }}%</span>
              <span class="shrink-0 text-foreground/60" v-if="otherJobs.length">+{{ otherJobs.length }}</span>
              <button v-if="activeJob.job === 'posters'" class="grid size-5 shrink-0 place-items-center rounded-full hover:bg-destructive hover:text-white" title="Stop fetching posters" @click="api.cancelPosters()"><X class="size-3" /></button>
              <div class="absolute inset-x-0 bottom-0 h-[2px] bg-foreground/10"><div class="bg-brand h-full transition-[width] duration-300" :class="{ 'animate-pulse': activeJob.total === 0 }" :style="{ width: (activeJob.total > 0 ? jobPct : 100) + '%' }"></div></div>
            </div>
            <Button variant="ghost" size="icon" class="rounded-full" :title="activeJob ? jobTitle : 'Sync library: rescan folders and refresh'" @click="syncLibrary">
              <RefreshCw :class="{ 'animate-spin': syncing || activeJob }" />
            </Button>
            <Button variant="ghost" size="icon" class="rounded-full" :title="dark ? 'Light mode' : 'Dark mode'" @click="toggleTheme">
              <Sun v-if="dark" /><Moon v-else />
            </Button>
            <Button variant="ghost" size="icon" class="rounded-full" :class="{ 'bg-accent': page === 'settings' && !openId && !search }" title="Settings" @click="go('settings')">
              <Settings />
            </Button>
          </div>
        </div>
      </header>

      <main ref="scroller" class="flex-1 overflow-y-auto overflow-x-hidden" @scroll.passive="onScroll">
        <SearchView v-if="search.trim()" class="mx-auto max-w-[1700px] px-10 pb-12 pt-24" :query="search" @open="openItem" />
        <SeriesView v-else-if="openId !== null" ref="detail" :id="openId" @back="back" @replaced="(id) => (openId = id)" />
        <HomeView v-else-if="page === 'home'" ref="home" @open="openItem" @go="go" />
        <div v-else class="mx-auto max-w-[1700px] px-10 pb-12 pt-24">
          <LibraryView v-if="page === 'movies'" ref="library" kind="movie" @open="openItem" @save-smart="addSmartList" />
          <LibraryView v-else-if="page === 'series'" ref="library" kind="series" @open="openItem" @save-smart="addSmartList" />
          <LibraryView v-else-if="page === 'smart' && activeSmart" ref="library" :kind="activeSmart.kind === 'all' ? undefined : activeSmart.kind" :smart="activeSmart" @open="openItem" @save-smart="addSmartList" />
          <CollectionsView v-else-if="page === 'collections'" ref="collectionsRef" @open="openItem" />
          <HistoryView v-else-if="page === 'history'" ref="history" @open="openItem" />
          <StatsView v-else-if="page === 'stats'" ref="statsRef" @open="openItem" />
          <DuplicatesView v-else-if="page === 'duplicates'" ref="duplicates" @open="openItem" />
          <DownloadsView v-else-if="page === 'downloads'" ref="downloadsRef" @go="go" />
          <TmdbView v-else-if="page === 'tmdb'" @open="openItem" @changed="refreshAll" />
          <SettingsView v-else @scanned="onScanned" @go="go" />
        </div>
      </main>

      <QuickSearch v-model:open="quickOpen" @open-item="openItem" @go="go" @toggle-theme="toggleTheme" @rescan="rescanFromPalette" @search="(q) => (search = q)" />

      <Toaster position="bottom-right" :theme="dark ? 'dark' : 'light'" rich-colors close-button />

      <div v-if="upNext" class="glass fixed bottom-6 right-6 z-50 flex items-center gap-3 rounded-2xl border border-primary/30 p-3 pl-4 shadow-2xl animate-in slide-in-from-bottom-4">
        <div class="max-w-xs">
          <div class="text-xs text-muted-foreground">Up next in {{ upNext.seconds }}s</div>
          <div class="truncate font-semibold">{{ upNext.label }}</div>
        </div>
        <Button variant="brand" size="sm" class="rounded-full" @click="playUpNext"><Play class="fill-current" /> Play now</Button>
        <Button size="icon-sm" variant="ghost" class="rounded-full" @click="cancelUpNext"><X /></Button>
      </div>
    </div>
  </TooltipProvider>
</template>
