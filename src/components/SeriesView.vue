<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { toast } from "vue-sonner";
import { openUrl } from "@tauri-apps/plugin-opener";
import {
  ArrowLeft, Play, Check, RotateCcw, Timer, Info, FolderOpen, ExternalLink, Clapperboard, RefreshCw, Search, Star, Subtitles, X, Tag as TagIcon, Layers, Plus, FilePen,
} from "@lucide/vue";
import { api, backdropSrc, posterSrc, subtitleCount, type Details, type Episode, type MediaItem, type Tag, type TmdbMatch } from "../lib/api";
import { copyText, dateFromUnix, episodeCode, episodeTitle, fileSize, fileSizeExact, folderOf, hms, parseHms, relativeTime } from "../lib/format";
import RenameDialog from "./RenameDialog.vue";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { Separator } from "@/components/ui/separator";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuSeparator, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";

const props = defineProps<{ id: number }>();
const emit = defineEmits<{ back: [] }>();

const item = ref<MediaItem | null>(null);
const episodes = ref<Episode[]>([]);
const details = ref<Details | null>(null);
const detailsBusy = ref(false);
const editing = ref<number | null>(null);
const editValue = ref("");
const openDetails = ref<number | null>(null);
const showAllCast = ref(false);

async function load() {
  item.value = await api.getMediaItem(props.id);
  episodes.value = await api.listEpisodes(props.id);
  loadDetails(false);
  loadTagData();
}

async function loadDetails(refresh: boolean) {
  if (!item.value?.tmdb_id) { details.value = null; return; }
  detailsBusy.value = true;
  try {
    details.value = await api.getDetails(props.id, refresh);
    if (refresh) {
      await api.fetchEpisodeTitles(props.id).catch(() => 0);
      episodes.value = await api.listEpisodes(props.id);
      toast.success("Details refreshed");
    }
  } catch (e) {
    if (refresh) toast.error(String(e));
  } finally {
    detailsBusy.value = false;
  }
}

// ---- derived ----
const mainEpisodes = computed(() => episodes.value.filter((e) => !e.extra));
const seasons = computed(() => {
  const map = new Map<number | null, Episode[]>();
  for (const e of mainEpisodes.value) map.set(e.season, [...(map.get(e.season) ?? []), e]);
  return [...map.entries()].sort((a, b) => (a[0] ?? 999) - (b[0] ?? 999));
});
const extras = computed(() => {
  const map = new Map<string, Episode[]>();
  for (const e of episodes.value) if (e.extra) map.set(e.extra, [...(map.get(e.extra) ?? []), e]);
  return [...map.entries()].sort((a, b) => a[0].localeCompare(b[0]));
});
const seasonInfo = (n: number | null) => (n == null ? null : details.value?.seasons.find((s) => s.season_number === n) ?? null);
const allDone = computed(() => mainEpisodes.value.length > 0 && mainEpisodes.value.every((e) => e.completed));
const primary = computed(() => {
  const eps = mainEpisodes.value;
  return eps.find((e) => e.position_secs > 0 && !e.completed) ?? eps.find((e) => !e.completed) ?? eps[0] ?? null;
});
const primaryLabel = computed(() => {
  const ep = primary.value;
  if (!ep) return "No file";
  const code = item.value?.kind === "series" ? episodeCode(ep) + " " : "";
  if (ep.position_secs > 0 && !ep.completed) return `Resume ${code}at ${hms(ep.position_secs)}`;
  if (item.value?.kind === "series") return allDone.value ? "Play again from S01E01" : `Play ${episodeCode(ep)}`;
  return allDone.value ? "Watch again" : "Play";
});
const year = computed(() => details.value?.release_date?.slice(0, 4) ?? item.value?.year ?? null);
const runtime = computed(() => { const m = details.value?.runtime_min; if (!m) return null; return m >= 60 ? `${Math.floor(m / 60)}h ${m % 60}m` : `${m}m`; });
const visibleCast = computed(() => (showAllCast.value ? details.value?.cast ?? [] : (details.value?.cast ?? []).slice(0, 8)));
const itemFolder = computed(() => {
  const first = episodes.value[0];
  if (!first) return null;
  const folder = folderOf(first.path);
  return item.value?.kind === "series" && /[\\/](season|series|s)\s*\d+$/i.test(folder) ? folderOf(folder) : folder;
});
const quality = (f: Episode) => /\b(2160p|1080p|720p|480p)\b/i.exec(f.file_name)?.[1] ?? null;

// ---- actions ----
async function play(ep: Episode | null) {
  if (!ep) return;
  try {
    const tracked = await api.playEpisode(ep.id);
    if (!tracked) toast.warning("Opened with the default app. Choose a player in Settings to track progress.");
    else if (ep.position_secs > 0 && !ep.completed) toast(`Resuming at ${hms(ep.position_secs)}`);
  } catch (e) {
    toast.error(String(e));
  }
}
async function toggleWatched(ep: Episode) {
  if (ep.completed) await api.clearProgress(ep.id); else await api.setProgress(ep.id, 0, true);
  await load();
}
function startEdit(ep: Episode) { editing.value = ep.id; editValue.value = ep.position_secs > 0 ? hms(ep.position_secs) : ""; }
async function saveEdit(ep: Episode) {
  const secs = parseHms(editValue.value);
  if (secs == null) { toast.error("Enter a time like 23:14 or 1:05:00"); return; }
  await api.setProgress(ep.id, secs, false);
  editing.value = null;
  await load();
}
async function markAll(watched: boolean) { await api.setItemWatched(props.id, watched); await load(); }
const toggleDetails = (ep: Episode) => { openDetails.value = openDetails.value === ep.id ? null : ep.id; };
async function reveal(path: string) { try { await api.revealPath(path); } catch (e) { toast.error(String(e)); } }
async function copy(text: string, what: string) { toast((await copyText(text)) ? `${what} copied` : "Could not access the clipboard"); }
async function open(url: string) { try { await openUrl(url); } catch (e) { toast.error(String(e)); } }

// ---- tags & collections ----
const tagInput = ref("");
const allTags = ref<string[]>([]);
const allCollections = ref<Tag[]>([]);
const newCollectionOpen = ref(false);
const newCollectionName = ref("");
const itemTags = computed(() => (item.value?.tags ? item.value.tags.split(", ") : []));
const itemCollections = computed(() => (item.value?.collections ? item.value.collections.split(", ") : []));
async function loadTagData() {
  allTags.value = (await api.listTags("tag")).map((t) => t.name);
  allCollections.value = await api.listTags("collection");
}
async function addTag() {
  const t = tagInput.value.trim();
  if (!t) return;
  if (!itemTags.value.includes(t)) await api.setItemTags(props.id, [...itemTags.value, t]);
  tagInput.value = "";
  await load();
}
async function removeTag(t: string) { await api.setItemTags(props.id, itemTags.value.filter((x) => x !== t)); await load(); }
async function addToCollection(id: number) { await api.addToCollection(id, props.id); await load(); toast.success("Added to collection"); }
async function createCollection() {
  const name = newCollectionName.value.trim();
  if (!name) return;
  const id = await api.createTag("collection", name);
  newCollectionOpen.value = false;
  newCollectionName.value = "";
  await addToCollection(id);
}
async function removeFromCollection(name: string) {
  const c = allCollections.value.find((x) => x.name === name);
  if (c) { await api.removeFromCollection(c.id, props.id); await load(); }
}

// ---- category ----
const categoryEditing = ref(false);
const categoryValue = ref("");
const knownCategories = ref<string[]>([]);
async function startCategoryEdit() { knownCategories.value = await api.listCategories(); categoryValue.value = item.value?.category ?? ""; categoryEditing.value = true; }
async function saveCategory() { await api.setItemCategory(props.id, categoryValue.value.trim() || null); categoryEditing.value = false; await load(); }

// ---- TMDB match ----
const matchOpen = ref(false);
const renameOpen = ref(false);
const matchQuery = ref("");
const matchResults = ref<TmdbMatch[]>([]);
const matchBusy = ref(false);
async function openMatch() { matchQuery.value = item.value?.title ?? ""; matchResults.value = []; matchOpen.value = true; await runMatchSearch(); }
async function runMatchSearch() {
  if (!item.value || !matchQuery.value.trim()) return;
  matchBusy.value = true;
  try { matchResults.value = await api.searchTmdb(item.value.kind, matchQuery.value.trim(), null); }
  catch (e) { toast.error(String(e)); } finally { matchBusy.value = false; }
}
async function pickMatch(m: TmdbMatch) {
  if (!item.value) return;
  matchBusy.value = true;
  try { await api.applyTmdbMatch(item.value.id, m); matchOpen.value = false; await load(); toast.success(`Matched to ${m.title}`); }
  catch (e) { toast.error(String(e)); } finally { matchBusy.value = false; }
}

watch(() => props.id, load);
onMounted(load);
defineExpose({ reload: load });
</script>

<template>
  <div v-if="!item" class="space-y-4 px-10 pt-24"><Skeleton class="h-[60vh] rounded-3xl" /><Skeleton class="h-40 rounded-xl" /></div>
  <div v-else>
    <!-- Hero -->
    <div class="relative min-h-[600px] overflow-hidden">
      <img v-if="details && backdropSrc(details)" :src="backdropSrc(details)!" alt="" draggable="false"
           class="absolute inset-0 h-full w-full object-cover object-[center_22%] animate-in fade-in duration-700" />
      <img v-else-if="posterSrc(item)" :src="posterSrc(item)!" alt="" draggable="false"
           class="absolute inset-0 h-full w-full scale-125 object-cover opacity-45 blur-3xl" />
      <div class="hero-fade absolute inset-0"></div>

      <div class="relative z-10 mx-auto max-w-[1920px] px-10 pb-12 pt-20">
        <div class="mb-10 flex flex-wrap items-center gap-2">
          <Button variant="soft" size="sm" class="rounded-full" @click="emit('back')"><ArrowLeft /> Back</Button>
          <div class="flex-1"></div>
          <template v-if="categoryEditing">
            <Input v-model="categoryValue" list="cats" placeholder="Category" class="h-8 w-44 rounded-full" autofocus @keyup.enter="saveCategory" @keyup.esc="categoryEditing = false" />
            <datalist id="cats"><option v-for="c in knownCategories" :key="c" :value="c" /></datalist>
            <Button size="sm" class="rounded-full" @click="saveCategory">Save</Button>
            <Button size="sm" variant="ghost" class="rounded-full" @click="categoryEditing = false">Cancel</Button>
          </template>
          <Button v-else variant="soft" size="sm" class="rounded-full" title="Change category" @click="startCategoryEdit">{{ item.category || "No category" }}</Button>
          <Button variant="soft" size="sm" class="rounded-full" @click="openMatch"><Search /> Fix match</Button>
          <Button v-if="item.tmdb_id" variant="soft" size="sm" class="rounded-full" :disabled="detailsBusy" @click="loadDetails(true)"><RefreshCw :class="{ 'animate-spin': detailsBusy }" /> Refresh</Button>
          <Button v-if="item.tmdb_id" variant="soft" size="sm" class="rounded-full" @click="renameOpen = true"><FilePen /> Rename files</Button>
          <Button variant="soft" size="sm" class="rounded-full" @click="markAll(!allDone)"><component :is="allDone ? RotateCcw : Check" /> {{ allDone ? "Mark all unwatched" : "Mark all watched" }}</Button>
        </div>

        <div class="flex items-end gap-10">
          <img v-if="posterSrc(item)" :src="posterSrc(item)!" :alt="item.title" class="poster-shadow hidden h-[390px] w-[260px] shrink-0 rounded-2xl object-cover ring-1 ring-white/10 lg:block" />
          <div v-else class="poster-shadow hidden h-[390px] w-[260px] shrink-0 place-items-center rounded-2xl bg-muted text-6xl font-black text-muted-foreground lg:grid">{{ item.title.slice(0, 1) }}</div>

          <div class="min-w-0 flex-1 pb-1">
            <div class="text-brand mb-2 text-xs font-extrabold uppercase tracking-[0.24em]">{{ item.kind === "series" ? "Series" : "Movie" }}<template v-if="item.category"> · {{ item.category }}</template></div>
            <h1 class="text-[clamp(2.4rem,4.6vw,4.25rem)] font-black leading-[0.98] tracking-[-0.04em]">{{ details?.title || item.title }}</h1>
            <div class="mt-1 text-lg text-foreground/60" v-if="details?.original_title && details.original_title !== details.title">{{ details.original_title }}</div>

            <div class="mt-5 flex flex-wrap items-center gap-x-3 gap-y-2 text-sm font-medium text-foreground/80">
              <span class="bg-brand flex items-center gap-1 rounded-full px-2.5 py-0.5 text-xs font-extrabold" v-if="details?.rating"><Star class="size-3 fill-current" />{{ details.rating.toFixed(1) }}</span>
              <span class="text-foreground/55" v-if="details?.vote_count">{{ details.vote_count.toLocaleString() }} votes</span>
              <span v-if="year">{{ year }}</span>
              <span class="rounded border border-foreground/25 px-1.5 text-xs" v-if="details?.certification">{{ details.certification }}</span>
              <span v-if="runtime">{{ runtime }}<template v-if="item.kind === 'series'"> / ep</template></span>
              <span v-if="item.kind === 'series' && details?.number_of_seasons">{{ details.number_of_seasons }} season{{ details.number_of_seasons === 1 ? "" : "s" }} · {{ details.number_of_episodes }} episodes</span>
              <span v-if="details?.status && item.kind === 'series'">{{ details.status }}</span>
              <span v-if="details?.language" class="uppercase">{{ details.language }}</span>
            </div>
            <div class="mt-4 flex flex-wrap gap-2" v-if="details?.genres.length">
              <span v-for="g in details.genres" :key="g" class="rounded-full border border-foreground/12 bg-foreground/[0.06] px-3 py-1 text-xs font-medium backdrop-blur">{{ g }}</span>
            </div>

            <p class="mt-5 text-lg font-medium italic text-foreground/70" v-if="details?.tagline">“{{ details.tagline }}”</p>
            <p class="mt-2 max-w-3xl text-[15px] leading-7 text-foreground/80" v-if="details?.overview || item.overview">{{ details?.overview || item.overview }}</p>

            <div class="mt-7 flex flex-wrap items-center gap-3">
              <Button variant="brand" size="xl" :disabled="!primary || !primary.available" @click="play(primary)"><Play class="fill-current" /> {{ primaryLabel }}</Button>
              <Button variant="soft" size="lg" class="h-12 rounded-full px-5" v-if="details?.trailer_youtube" @click="open(`https://www.youtube.com/watch?v=${details.trailer_youtube}`)"><Clapperboard /> Trailer</Button>
              <Button variant="soft" size="icon-lg" class="size-12 rounded-full" v-if="details?.imdb_id" title="IMDb" @click="open(`https://www.imdb.com/title/${details.imdb_id}/`)"><span class="text-[11px] font-black">IMDb</span></Button>
              <Button variant="soft" size="icon-lg" class="size-12 rounded-full" v-if="details" title="TMDb" @click="open(details.tmdb_url)"><ExternalLink /></Button>
              <Button variant="soft" size="icon-lg" class="size-12 rounded-full" v-if="itemFolder" :title="itemFolder" @click="reveal(episodes[0].path)"><FolderOpen /></Button>
            </div>
            <p class="mt-4 text-sm text-muted-foreground" v-if="!item.tmdb_id">Not matched to TMDB yet. Click <strong>Fix match</strong> for the poster, cast and description.</p>
          </div>
        </div>
      </div>
    </div>

    <div class="mx-auto max-w-[1920px] space-y-12 px-10 pb-16">
      <!-- People and tags -->
      <div class="grid gap-x-12 gap-y-6 lg:grid-cols-2">
        <div class="grid content-start gap-2 text-sm" v-if="details">
          <div v-if="details.creators.length"><span class="inline-block w-28 text-muted-foreground">Created by</span><span class="font-medium">{{ details.creators.join(", ") }}</span></div>
          <div v-if="details.directors.length"><span class="inline-block w-28 text-muted-foreground">Director</span><span class="font-medium">{{ details.directors.join(", ") }}</span></div>
          <div v-if="details.writers.length"><span class="inline-block w-28 text-muted-foreground">Writers</span><span class="font-medium">{{ details.writers.slice(0, 4).join(", ") }}</span></div>
          <div v-if="details.companies.length"><span class="inline-block w-28 text-muted-foreground">{{ item.kind === "series" ? "Network" : "Studio" }}</span><span class="font-medium">{{ details.companies.join(", ") }}</span></div>
          <div v-if="details.release_date"><span class="inline-block w-28 text-muted-foreground">{{ item.kind === "series" ? "Aired" : "Released" }}</span><span class="font-medium">{{ details.release_date }}<template v-if="details.last_air_date && details.last_air_date !== details.release_date"> → {{ details.last_air_date }}</template></span></div>
        </div>
        <div class="grid content-start gap-3">
          <div class="flex flex-wrap items-center gap-1.5 text-sm">
            <TagIcon class="mr-1 size-4 text-muted-foreground" />
            <span v-for="t in itemTags" :key="t" class="flex items-center gap-1 rounded-full bg-secondary py-0.5 pl-3 pr-1 text-xs font-medium">{{ t }} <button class="rounded-full p-0.5 hover:bg-destructive hover:text-white" @click="removeTag(t)"><X class="size-3" /></button></span>
            <Input v-model="tagInput" list="all-tags" placeholder="+ tag" class="h-7 w-32 rounded-full text-xs" @keyup.enter="addTag" @blur="addTag" />
            <datalist id="all-tags"><option v-for="t in allTags" :key="t" :value="t" /></datalist>
          </div>
          <div class="flex flex-wrap items-center gap-1.5 text-sm">
            <Layers class="mr-1 size-4 text-muted-foreground" />
            <span v-for="c in itemCollections" :key="c" class="flex items-center gap-1 rounded-full bg-secondary py-0.5 pl-3 pr-1 text-xs font-medium">{{ c }} <button class="rounded-full p-0.5 hover:bg-destructive hover:text-white" @click="removeFromCollection(c)"><X class="size-3" /></button></span>
            <DropdownMenu>
              <DropdownMenuTrigger as-child><Button variant="outline" size="xs" class="rounded-full"><Plus /> collection</Button></DropdownMenuTrigger>
              <DropdownMenuContent align="start">
                <DropdownMenuItem v-for="c in allCollections.filter((x) => !itemCollections.includes(x.name))" :key="c.id" @select="addToCollection(c.id)">{{ c.name }}</DropdownMenuItem>
                <DropdownMenuSeparator v-if="allCollections.length" />
                <DropdownMenuItem @select="newCollectionOpen = true"><Plus /> New collection…</DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </div>
        </div>
      </div>

      <!-- Cast -->
      <section v-if="details?.cast.length">
        <h2 class="section-title mb-5 text-[1.3rem]">Cast &amp; crew</h2>
        <div class="no-scrollbar flex gap-6 overflow-x-auto pb-2">
          <div v-for="c in visibleCast" :key="c.name + c.character" class="w-[112px] shrink-0 text-center">
            <Avatar class="mx-auto size-24 ring-2 ring-border transition-transform hover:scale-105">
              <AvatarImage v-if="c.photo_url" :src="c.photo_url" :alt="c.name" class="object-cover" />
              <AvatarFallback class="bg-gradient-to-br from-secondary to-accent text-2xl font-bold text-muted-foreground">{{ c.name.split(" ").map((w) => w[0]).slice(0, 2).join("") }}</AvatarFallback>
            </Avatar>
            <div class="mt-2.5 text-sm font-semibold leading-tight">{{ c.name }}</div>
            <div class="mt-0.5 text-xs leading-tight text-muted-foreground" v-if="c.character">{{ c.character }}</div>
          </div>
        </div>
        <Button variant="link" size="sm" class="px-0" v-if="details.cast.length > 8" @click="showAllCast = !showAllCast">{{ showAllCast ? "Show fewer" : `Show all ${details.cast.length}` }}</Button>
      </section>

      <!-- Episodes / files -->
      <template v-for="[season, eps] in (item.kind === 'series' ? seasons : [[null, mainEpisodes] as [number | null, Episode[]]])" :key="season ?? 'files'">
        <section>
          <div class="mb-4 flex items-baseline gap-3">
            <h2 class="section-title text-[1.3rem]">
              {{ item.kind === "series" ? (season != null ? seasonInfo(season)?.name || `Season ${season}` : "Unsorted") : (eps.length === 1 ? "File" : "Files") }}
            </h2>
            <span class="text-xs text-muted-foreground" v-if="seasonInfo(season)">{{ eps.length }} of {{ seasonInfo(season)!.episode_count }} on disk<template v-if="seasonInfo(season)!.air_date"> · {{ seasonInfo(season)!.air_date!.slice(0, 4) }}</template></span>
          </div>
          <p class="mb-3 max-w-3xl text-sm text-muted-foreground" v-if="seasonInfo(season)?.overview">{{ seasonInfo(season)!.overview }}</p>

          <div class="space-y-1.5">
            <template v-for="ep in eps" :key="ep.id">
              <div class="group/ep card-shadow grid items-center gap-4 rounded-2xl border border-transparent bg-card p-3 transition-all hover:border-primary/30 hover:bg-accent/40"
                   :class="[item.kind === 'series' ? 'grid-cols-[200px_1fr_auto]' : 'grid-cols-[64px_1fr_auto]', { 'opacity-60': !ep.available, 'border-primary/50 rounded-b-none': openDetails === ep.id }]"
                   @dblclick="play(ep)">
                <div v-if="item.kind === 'series'" class="relative aspect-video cursor-pointer overflow-hidden rounded-xl bg-muted" @click="play(ep)" @dblclick.stop>
                  <img v-if="ep.still_path" :src="ep.still_path" loading="lazy" alt="" class="h-full w-full object-cover transition-transform duration-500 group-hover/ep:scale-105" />
                  <div class="absolute inset-0 grid place-items-center bg-black/35 opacity-0 transition-opacity group-hover/ep:opacity-100"><span class="bg-brand grid size-10 place-items-center rounded-full shadow-lg"><Play class="ml-0.5 size-4 fill-current" /></span></div>
                  <div v-if="ep.duration_secs && ep.position_secs > 0 && !ep.completed" class="absolute inset-x-0 bottom-0 h-1 bg-black/50"><div class="bg-brand h-full" :style="{ width: (100 * ep.position_secs / ep.duration_secs) + '%' }"></div></div>
                  <div v-if="ep.completed" class="bg-brand absolute right-1.5 top-1.5 grid size-5 place-items-center rounded-full"><Check class="size-3" stroke-width="3" /></div>
                </div>
                <div v-else class="text-center text-sm font-semibold" :class="ep.completed ? 'text-success' : 'text-primary'">{{ quality(ep) ?? (ep.completed ? "✓" : "▶") }}</div>

                <div class="min-w-0">
                  <div class="truncate text-[15px] font-semibold">
                    <span class="mr-2 font-bold text-primary" v-if="item.kind === 'series'">{{ episodeCode(ep) }}</span>{{ item.kind === "series" ? episodeTitle(ep) : ep.file_name }}
                    <span class="ml-2 text-xs text-warning" v-if="ep.rating">★ {{ ep.rating.toFixed(1) }}</span>
                    <span class="ml-2 text-xs text-muted-foreground" v-if="ep.air_date">{{ ep.air_date }}</span>
                  </div>
                  <div class="line-clamp-2 text-xs leading-snug text-muted-foreground" v-if="ep.overview">{{ ep.overview }}</div>
                  <div class="mt-0.5 flex flex-wrap items-center gap-x-2 text-xs text-muted-foreground">
                    <span v-if="ep.duration_secs">{{ hms(ep.duration_secs) }}</span><span>{{ fileSize(ep.size) }}</span>
                    <Tooltip v-if="subtitleCount(ep)"><TooltipTrigger as-child><span class="flex items-center gap-0.5 text-foreground/80"><Subtitles class="size-3.5" /> {{ subtitleCount(ep) }}</span></TooltipTrigger><TooltipContent><div v-for="s in ep.subtitles!.split('|')" :key="s">{{ s.split(/[\\/]/).pop() }}</div></TooltipContent></Tooltip>
                    <span v-if="ep.completed">· watched {{ relativeTime(ep.last_watched) }}</span>
                    <span v-else-if="ep.position_secs > 0">· paused at {{ hms(ep.position_secs) }} · {{ relativeTime(ep.last_watched) }}</span>
                    <span v-if="!ep.available" class="text-destructive">· file not available</span>
                  </div>
                </div>

                <div class="flex items-center gap-1">
                  <template v-if="editing === ep.id">
                    <Input v-model="editValue" placeholder="mm:ss" class="h-8 w-24" autofocus @keyup.enter="saveEdit(ep)" @keyup.esc="editing = null" />
                    <Button size="sm" @click="saveEdit(ep)">Save</Button>
                    <Button size="sm" variant="ghost" @click="editing = null">Cancel</Button>
                  </template>
                  <template v-else>
                    <Tooltip><TooltipTrigger as-child><Button size="icon-sm" :variant="openDetails === ep.id ? 'default' : 'ghost'" @click="toggleDetails(ep)"><Info /></Button></TooltipTrigger><TooltipContent>File details</TooltipContent></Tooltip>
                    <Tooltip><TooltipTrigger as-child><Button size="icon-sm" variant="ghost" @click="startEdit(ep)"><Timer /></Button></TooltipTrigger><TooltipContent>Set paused time</TooltipContent></Tooltip>
                    <Tooltip><TooltipTrigger as-child><Button size="icon-sm" variant="ghost" @click="toggleWatched(ep)"><component :is="ep.completed ? RotateCcw : Check" /></Button></TooltipTrigger><TooltipContent>{{ ep.completed ? "Mark unwatched" : "Mark watched" }}</TooltipContent></Tooltip>
                    <Button size="sm" variant="brand" class="rounded-full" :disabled="!ep.available" @click="play(ep)" @dblclick.stop><Play class="fill-current" /> {{ ep.position_secs > 0 && !ep.completed ? "Resume" : "Play" }}</Button>
                  </template>
                </div>
              </div>

              <div v-if="openDetails === ep.id" class="-mt-1.5 grid grid-cols-[110px_1fr] gap-x-4 gap-y-1.5 rounded-b-2xl border border-t-0 border-primary/50 bg-accent/40 px-4 py-3 text-sm">
                <span class="text-muted-foreground">File</span><span class="break-all select-text">{{ ep.file_name }}</span>
                <span class="text-muted-foreground">Folder</span><span class="break-all select-text">{{ folderOf(ep.path) }}</span>
                <span class="text-muted-foreground">Full path</span><span class="break-all select-text">{{ ep.path }}</span>
                <span class="text-muted-foreground">Size</span><span>{{ fileSizeExact(ep.size) }}</span>
                <span class="text-muted-foreground">Modified</span><span>{{ dateFromUnix(ep.modified) }}</span>
                <span class="text-muted-foreground">Duration</span><span>{{ ep.duration_secs ? hms(ep.duration_secs) : "unknown" }}</span>
                <template v-if="ep.subtitles"><span class="text-muted-foreground">Subtitles</span><span class="break-all select-text">{{ ep.subtitles.split("|").map((s) => s.split(/[\\/]/).pop()).join(", ") }}</span></template>
                <span class="text-muted-foreground">Status</span><span>{{ ep.available ? "Available" : "Not available (drive disconnected or file moved)" }}</span>
                <div class="col-span-2 mt-1 flex gap-2">
                  <Button size="sm" variant="outline" :disabled="!ep.available" @click="reveal(ep.path)"><FolderOpen /> Show in Explorer</Button>
                  <Button size="sm" variant="outline" @click="copy(ep.path, 'Path')">Copy path</Button>
                  <Button size="sm" variant="outline" @click="copy(folderOf(ep.path), 'Folder')">Copy folder</Button>
                  <Button size="sm" variant="outline" v-if="ep.position_secs > 0 || ep.completed" @click="api.clearProgress(ep.id).then(load)">Reset progress</Button>
                </div>
              </div>
            </template>
          </div>
        </section>
      </template>

      <!-- Extras -->
      <section v-if="extras.length">
        <h2 class="section-title mb-4 text-[1.3rem]">Extras <span class="text-sm font-normal text-muted-foreground">not counted as episodes</span></h2>
        <div v-for="[label, list] in extras" :key="label" class="mb-4">
          <div class="mb-1.5 text-xs font-medium text-muted-foreground">{{ label }}</div>
          <div class="space-y-1.5">
            <div v-for="ep in list" :key="ep.id" class="card-shadow grid grid-cols-[32px_1fr_auto] items-center gap-3 rounded-2xl bg-card px-3 py-2.5" :class="{ 'opacity-60': !ep.available }" @dblclick="play(ep)">
              <div class="text-center" :class="ep.completed ? 'text-success' : 'text-primary'"><component :is="ep.completed ? Check : Play" class="mx-auto size-4" /></div>
              <div class="min-w-0">
                <div class="truncate font-medium" :title="ep.path">{{ ep.file_name.replace(/\.[^.]+$/, "") }}</div>
                <div class="text-xs text-muted-foreground"><span v-if="ep.duration_secs">{{ hms(ep.duration_secs) }} · </span>{{ fileSize(ep.size) }}<span v-if="ep.position_secs > 0 && !ep.completed"> · paused at {{ hms(ep.position_secs) }}</span></div>
              </div>
              <div class="flex items-center gap-1">
                <Button size="icon-sm" variant="ghost" :disabled="!ep.available" title="Show in Explorer" @click="reveal(ep.path)"><FolderOpen /></Button>
                <Button size="icon-sm" variant="ghost" @click="toggleWatched(ep)"><component :is="ep.completed ? RotateCcw : Check" /></Button>
                <Button size="sm" :disabled="!ep.available" @click="play(ep)" @dblclick.stop><Play class="fill-current" /> Play</Button>
              </div>
            </div>
          </div>
        </div>
      </section>

      <Separator />
      <p class="text-xs text-muted-foreground" v-if="details">Data from TMDB · fetched {{ details.fetched_at.slice(0, 10) }}</p>
    </div>

    <RenameDialog v-model:open="renameOpen" :ids="[props.id]" @done="load" />

    <!-- Fix match -->
    <Dialog v-model:open="matchOpen">
      <DialogContent class="max-w-2xl">
        <DialogHeader><DialogTitle>Match on TMDB</DialogTitle></DialogHeader>
        <div class="flex gap-2">
          <Input v-model="matchQuery" placeholder="Search TMDB…" autofocus @keyup.enter="runMatchSearch" />
          <Button :disabled="matchBusy" @click="runMatchSearch"><Search /> Search</Button>
        </div>
        <div class="max-h-[60vh] space-y-2 overflow-y-auto">
          <p v-if="matchBusy" class="text-sm text-muted-foreground">Working…</p>
          <p v-else-if="matchResults.length === 0" class="text-sm text-muted-foreground">No results.</p>
          <button v-for="m in matchResults" :key="m.tmdb_id" class="flex w-full items-center gap-3 rounded-lg border p-2 text-left hover:border-primary hover:bg-accent" @click="pickMatch(m)">
            <img v-if="m.poster_url" :src="m.poster_url" alt="" class="h-[69px] w-[46px] shrink-0 rounded object-cover" />
            <div v-else class="h-[69px] w-[46px] shrink-0 rounded bg-muted"></div>
            <div class="min-w-0">
              <div class="font-medium">{{ m.title }} <span class="text-muted-foreground" v-if="m.year">({{ m.year }})</span> <span class="text-xs text-warning" v-if="m.rating">★ {{ m.rating }}</span></div>
              <div class="line-clamp-2 text-xs text-muted-foreground" v-if="m.overview">{{ m.overview }}</div>
            </div>
          </button>
        </div>
      </DialogContent>
    </Dialog>

    <Dialog v-model:open="newCollectionOpen">
      <DialogContent class="max-w-sm">
        <DialogHeader><DialogTitle>New collection</DialogTitle></DialogHeader>
        <Input v-model="newCollectionName" placeholder="Name" autofocus @keyup.enter="createCollection" />
        <div class="flex justify-end gap-2"><Button variant="outline" @click="newCollectionOpen = false">Cancel</Button><Button @click="createCollection">Create and add</Button></div>
      </DialogContent>
    </Dialog>
  </div>
</template>
