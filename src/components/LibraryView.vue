<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { toast } from "vue-sonner";
import { Film, Tv, Sparkles, Save } from "@lucide/vue";
import { api, posterSrc, type MediaItem, type SmartList } from "../lib/api";
import { relativeTime } from "../lib/format";
import { useGridKeys } from "../composables/useGridKeys";
import MediaCard from "./MediaCard.vue";
import EmptyState from "./EmptyState.vue";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";

const props = defineProps<{ kind?: "movie" | "series"; smart?: SmartList }>();
const emit = defineEmits<{ open: [id: number]; saveSmart: [list: SmartList] }>();

const items = ref<MediaItem[]>([]);
const loading = ref(true);
const query = ref("");
const filter = ref<"all" | "unwatched" | "watched">("all");
const category = ref("all");
const genre = ref("all");
const tag = ref("all");
const root = ref<HTMLElement>();
useGridKeys(root);

type Sort = "title" | "year" | "added" | "watched" | "rating" | "size";
const SORT_KEY = "vortex-sort";
const sort = ref<Sort>(((): Sort => { try { const v = localStorage.getItem(SORT_KEY); if (v) return v as Sort; } catch {} return "title"; })());
watch(sort, (v) => { try { localStorage.setItem(SORT_KEY, v); } catch {} });

async function load() {
  items.value = await api.listMedia(props.kind);
  loading.value = false;
}

const distinct = (pick: (m: MediaItem) => string[]) =>
  computed(() => [...new Set(items.value.flatMap(pick).filter(Boolean))].sort((a, b) => a.localeCompare(b)));
const categories = distinct((m) => (m.category ? [m.category] : []));
const genres = distinct((m) => m.genres?.split(", ") ?? []);
const tags = distinct((m) => m.tags?.split(", ") ?? []);

const t = (s: string | null) => (s ? new Date(s.replace(" ", "T") + "Z").getTime() : 0);

const visible = computed(() => {
  const q = query.value.trim().toLowerCase();
  const s = props.smart;
  const now = Date.now();
  return items.value.filter((m) => {
    if (q && !m.title.toLowerCase().includes(q)) return false;
    const done = m.episode_count > 0 && m.watched_count >= m.episode_count;
    const f = s?.watched ?? filter.value;
    if (f === "watched" && !done) return false;
    if (f === "unwatched" && done) return false;
    const cat = s?.category ?? (category.value === "all" ? undefined : category.value);
    if (cat && m.category !== cat) return false;
    const g = s?.genre ?? (genre.value === "all" ? undefined : genre.value);
    if (g && !(m.genres?.split(", ") ?? []).includes(g)) return false;
    const tg = s?.tag ?? (tag.value === "all" ? undefined : tag.value);
    if (tg && !(m.tags?.split(", ") ?? []).includes(tg)) return false;
    if (s?.minYear && (m.year ?? 0) < s.minYear) return false;
    if (s?.maxYear && (m.year ?? 9999) > s.maxYear) return false;
    if (s?.minRating && (m.rating ?? 0) < s.minRating) return false;
    if (s?.untouchedDays && m.last_watched && now - t(m.last_watched) < s.untouchedDays * 86400000) return false;
    return true;
  });
});

const sorted = computed(() => {
  const list = [...visible.value];
  const byTitle = (a: MediaItem, b: MediaItem) => a.title.localeCompare(b.title);
  switch (sort.value) {
    case "year": return list.sort((a, b) => (b.year ?? 0) - (a.year ?? 0) || byTitle(a, b));
    case "added": return list.sort((a, b) => t(b.added_at) - t(a.added_at) || byTitle(a, b));
    case "watched": return list.sort((a, b) => t(b.last_watched) - t(a.last_watched) || byTitle(a, b));
    case "rating": return list.sort((a, b) => (b.rating ?? -1) - (a.rating ?? -1) || byTitle(a, b));
    case "size": return list.sort((a, b) => b.total_size - a.total_size);
    default: return list.sort(byTitle);
  }
});

function sub(m: MediaItem): string {
  const parts: string[] = [];
  if (m.year) parts.push(String(m.year));
  if (sort.value === "title" && m.genres) parts.push(m.genres.split(", ").slice(0, 2).join(", "));
  if (sort.value === "rating" && m.rating) parts.push(`★ ${m.rating.toFixed(1)}`);
  if (sort.value === "size") parts.push(`${(m.total_size / 1073741824).toFixed(1)} GB`);
  if (sort.value === "added" && m.added_at) parts.push(`added ${relativeTime(m.added_at)}`);
  if (sort.value === "watched" && m.last_watched) parts.push(`watched ${relativeTime(m.last_watched)}`);
  return parts.join(" · ");
}

function badge(m: MediaItem) {
  if (m.kind === "series") return `${m.watched_count} / ${m.episode_count} watched`;
  return m.watched_count > 0 ? "Watched" : "Unwatched";
}

async function playItem(m: MediaItem) {
  const eps = (await api.listEpisodes(m.id)).filter((e) => !e.extra);
  const ep = eps.find((e) => e.position_secs > 0 && !e.completed) ?? eps.find((e) => !e.completed) ?? eps[0];
  if (!ep) return;
  try {
    await api.playEpisode(ep.id);
  } catch (e) {
    toast.error(String(e));
  }
}

// ---- smart list from current filters ----
const saveOpen = ref(false);
const saveName = ref("");
const saveMinRating = ref("");
const saveUntouched = ref("");
const saveMinYear = ref("");
const hasFilters = computed(() => filter.value !== "all" || category.value !== "all" || genre.value !== "all" || tag.value !== "all");

function saveSmart() {
  const l: SmartList = {
    id: `${Date.now()}`,
    name: saveName.value.trim() || "Smart list",
    kind: props.kind ?? "all",
    watched: filter.value,
    category: category.value === "all" ? undefined : category.value,
    genre: genre.value === "all" ? undefined : genre.value,
    tag: tag.value === "all" ? undefined : tag.value,
    minRating: saveMinRating.value ? Number(saveMinRating.value) : undefined,
    untouchedDays: saveUntouched.value ? Number(saveUntouched.value) : undefined,
    minYear: saveMinYear.value ? Number(saveMinYear.value) : undefined,
  };
  emit("saveSmart", l);
  saveOpen.value = false;
  saveName.value = "";
}

const title = computed(() => props.smart?.name ?? (props.kind === "movie" ? "Movies" : "TV Series"));

watch(() => [props.kind, props.smart?.id], load);
onMounted(load);
defineExpose({ reload: load });
</script>

<template>
  <div ref="root">
    <div class="mb-5 flex flex-wrap items-center gap-2">
      <h1 class="mr-auto flex items-center gap-2 text-2xl font-semibold tracking-tight">
        <Sparkles v-if="smart" class="size-5 text-primary" />
        {{ title }}
        <span class="text-base font-normal text-muted-foreground">{{ sorted.length }}</span>
      </h1>
      <Input v-model="query" placeholder="Filter by title" class="h-9 w-52" />
      <template v-if="!smart">
        <Select v-model="filter">
          <SelectTrigger class="w-32"><SelectValue /></SelectTrigger>
          <SelectContent>
            <SelectItem value="all">All</SelectItem>
            <SelectItem value="unwatched">Unwatched</SelectItem>
            <SelectItem value="watched">Watched</SelectItem>
          </SelectContent>
        </Select>
        <Select v-model="category" v-if="categories.length > 1">
          <SelectTrigger class="w-40"><SelectValue placeholder="Category" /></SelectTrigger>
          <SelectContent><SelectItem value="all">All categories</SelectItem><SelectItem v-for="c in categories" :key="c" :value="c">{{ c }}</SelectItem></SelectContent>
        </Select>
        <Select v-model="genre" v-if="genres.length">
          <SelectTrigger class="w-40"><SelectValue placeholder="Genre" /></SelectTrigger>
          <SelectContent><SelectItem value="all">All genres</SelectItem><SelectItem v-for="g in genres" :key="g" :value="g">{{ g }}</SelectItem></SelectContent>
        </Select>
        <Select v-model="tag" v-if="tags.length">
          <SelectTrigger class="w-36"><SelectValue placeholder="Tag" /></SelectTrigger>
          <SelectContent><SelectItem value="all">All tags</SelectItem><SelectItem v-for="x in tags" :key="x" :value="x">{{ x }}</SelectItem></SelectContent>
        </Select>
      </template>
      <Select v-model="sort">
        <SelectTrigger class="w-44"><SelectValue /></SelectTrigger>
        <SelectContent>
          <SelectItem value="title">A → Z</SelectItem>
          <SelectItem value="year">Newest year</SelectItem>
          <SelectItem value="added">Recently added</SelectItem>
          <SelectItem value="watched">Recently watched</SelectItem>
          <SelectItem value="rating">Highest rated</SelectItem>
          <SelectItem value="size">Largest</SelectItem>
        </SelectContent>
      </Select>
      <Button v-if="!smart" variant="outline" size="sm" :title="hasFilters ? 'Save these filters as a smart list' : 'Set a filter first, or save a list with rating/age rules'" @click="saveOpen = true">
        <Save /> Smart list
      </Button>
    </div>

    <div v-if="smart" class="mb-4 flex flex-wrap gap-1.5">
      <Badge variant="secondary" v-if="smart.watched !== 'all'">{{ smart.watched }}</Badge>
      <Badge variant="secondary" v-if="smart.category">{{ smart.category }}</Badge>
      <Badge variant="secondary" v-if="smart.genre">{{ smart.genre }}</Badge>
      <Badge variant="secondary" v-if="smart.tag">#{{ smart.tag }}</Badge>
      <Badge variant="secondary" v-if="smart.minYear">from {{ smart.minYear }}</Badge>
      <Badge variant="secondary" v-if="smart.minRating">★ {{ smart.minRating }}+</Badge>
      <Badge variant="secondary" v-if="smart.untouchedDays">untouched {{ smart.untouchedDays }}+ days</Badge>
    </div>

    <div v-if="loading" class="grid grid-cols-[repeat(auto-fill,minmax(160px,1fr))] gap-4">
      <Skeleton v-for="i in 12" :key="i" class="aspect-[2/3] rounded-xl" />
    </div>
    <EmptyState v-else-if="items.length === 0" title="Nothing here yet" hint="Add a folder or drive in Settings. Scanning starts right away.">
      <template #icon><component :is="kind === 'movie' ? Film : Tv" class="size-6" /></template>
    </EmptyState>
    <EmptyState v-else-if="sorted.length === 0" title="No matches" hint="Try clearing a filter." />
    <div v-else class="grid grid-cols-[repeat(auto-fill,minmax(160px,1fr))] gap-4">
      <MediaCard v-for="m in sorted" :key="m.id" :title="m.title" :poster="posterSrc(m)" :subtitle="sub(m)"
                 :meta="category === 'all' && !smart ? m.category ?? undefined : undefined" :badge="badge(m)"
                 :done="m.episode_count > 0 && m.watched_count >= m.episode_count"
                 :progress="m.kind === 'series' && m.episode_count ? m.watched_count / m.episode_count : null"
                 playable @open="emit('open', m.id)" @play="playItem(m)" />
    </div>

    <Dialog v-model:open="saveOpen">
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Save as smart list</DialogTitle>
          <DialogDescription>Keeps the current filters and adds optional rules. It appears in the sidebar and updates itself.</DialogDescription>
        </DialogHeader>
        <div class="grid gap-3">
          <Input v-model="saveName" placeholder="Name, e.g. Unwatched 4K under 2h" autofocus @keyup.enter="saveSmart" />
          <div class="grid grid-cols-3 gap-2">
            <div><div class="mb-1 text-xs text-muted-foreground">Min rating</div><Input v-model="saveMinRating" placeholder="7.5" type="number" step="0.1" /></div>
            <div><div class="mb-1 text-xs text-muted-foreground">Untouched for days</div><Input v-model="saveUntouched" placeholder="60" type="number" /></div>
            <div><div class="mb-1 text-xs text-muted-foreground">From year</div><Input v-model="saveMinYear" placeholder="2015" type="number" /></div>
          </div>
          <div class="text-xs text-muted-foreground">
            Includes: {{ [filter !== 'all' ? filter : null, category !== 'all' ? category : null, genre !== 'all' ? genre : null, tag !== 'all' ? '#' + tag : null].filter(Boolean).join(', ') || 'no filters yet' }}
          </div>
        </div>
        <DialogFooter>
          <Button variant="outline" @click="saveOpen = false">Cancel</Button>
          <Button @click="saveSmart">Save</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  </div>
</template>
