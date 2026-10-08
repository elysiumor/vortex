<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { toast } from "vue-sonner";
import { Shuffle, Play, Info, Star, FolderPlus, Film } from "@lucide/vue";
import { api, backdropSrc, collectionPoster, posterSrc, type ContinueItem, type Details, type MediaItem, type Tag } from "../lib/api";
import { episodeCode, episodeTitle, hms, relativeTime } from "../lib/format";
import { useGridKeys } from "../composables/useGridKeys";
import MediaCard from "./MediaCard.vue";
import MediaRow from "./MediaRow.vue";
import { CarouselItem } from "@/components/ui/carousel";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";

const emit = defineEmits<{ open: [id: number]; go: [page: "collections" | "movies" | "series" | "settings"] }>();
const items = ref<ContinueItem[]>([]);
const all = ref<MediaItem[]>([]);
const collections = ref<Tag[]>([]);
const loading = ref(true);
const root = ref<HTMLElement>();
useGridKeys(root);

// ---- the billboard at the top ----
type Hero = { item: MediaItem; label: string; cont: ContinueItem | null };
const hero = ref<Hero | null>(null);
const heroDetails = ref<Details | null>(null);
const heroBackdrop = computed(() => (heroDetails.value ? backdropSrc(heroDetails.value) : null));

// Each pick gets a number; details that arrive for an older pick are dropped,
// or a quick second Shuffle showed one title's picture over another's buttons.
let heroSeq = 0;
async function loadHeroDetails(item: MediaItem) {
  const seq = ++heroSeq;
  heroDetails.value = null;
  if (!item.tmdb_id) return;
  try {
    const d = await api.getDetails(item.id);
    if (seq === heroSeq) heroDetails.value = d;
  } catch { /* the poster stands in */ }
}

async function chooseHero(random = false) {
  let pick: Hero | null = null;
  const first = items.value[0];
  if (!random && first) {
    const m = all.value.find((x) => x.id === first.episode.media_item_id);
    if (m) pick = { item: m, label: "Continue watching", cont: first };
  }
  if (!pick) {
    const fresh = all.value.filter((m) => m.watched_count < m.episode_count && m.tmdb_id);
    const pool = fresh.length ? fresh : all.value;
    if (pool.length) pick = { item: pool[Math.floor(Math.random() * pool.length)], label: random ? "Tonight's pick" : "Featured", cont: null };
    if (random && !fresh.length) toast("Everything is watched. Impressive.");
  }
  hero.value = pick;
  if (pick) await loadHeroDetails(pick.item);
  else { heroSeq++; heroDetails.value = null; }
}

/**
 * After a refresh: keep the same title on the billboard, but with fresh data.
 * A finished or dismissed "Continue watching" pick, or a title that left the
 * library, gets a new pick; a title matched to TMDb since gets its picture.
 */
async function refreshHero() {
  const h = hero.value;
  const item = h ? all.value.find((m) => m.id === h.item.id) : undefined;
  if (!h || !item) return chooseHero();
  if (h.cont) {
    const cont = items.value.find((c) => c.episode.media_item_id === item.id);
    if (!cont) return chooseHero();
    hero.value = { ...h, item, cont };
  } else {
    hero.value = { ...h, item };
  }
  if (item.tmdb_id !== h.item.tmdb_id || (item.tmdb_id && !heroDetails.value)) await loadHeroDetails(item);
}

const heroMeta = computed(() => {
  const h = hero.value;
  if (!h) return [];
  const d = heroDetails.value;
  const parts: string[] = [];
  if (h.item.year) parts.push(String(h.item.year));
  if (h.item.kind === "movie") parts.push(d?.runtime_min ? `${Math.floor(d.runtime_min / 60)}h ${d.runtime_min % 60}m` : "Movie");
  else parts.push(d?.number_of_seasons ? `${d.number_of_seasons} season${d.number_of_seasons === 1 ? "" : "s"}` : `${h.item.episode_count} episodes`);
  const genres = (d?.genres.length ? d.genres : (h.item.genres ?? "").split(", ")).filter(Boolean).slice(0, 3);
  return [...parts, ...genres];
});
const heroRating = computed(() => heroDetails.value?.rating ?? hero.value?.item.rating ?? null);
const heroOverview = computed(() => heroDetails.value?.overview || hero.value?.item.overview || "");
const heroResume = computed(() => {
  const c = hero.value?.cont;
  if (!c || !c.episode.duration_secs) return null;
  return {
    pct: Math.min(100, (100 * c.episode.position_secs) / c.episode.duration_secs),
    left: `${Math.max(1, Math.round((c.episode.duration_secs - c.episode.position_secs) / 60))} min left`,
    label: c.kind === "series" ? `${episodeCode(c.episode)} · ${episodeTitle(c.episode)}` : null,
  };
});

async function playHero() {
  const h = hero.value;
  if (!h) return;
  if (h.cont) await play(h.cont);
  else await playItem(h.item);
}

// ---- rows ----
const t = (s: string | null) => (s ? new Date(s.replace(" ", "T") + "Z").getTime() : 0);
const unwatched = (m: MediaItem) => m.watched_count < m.episode_count;
const recent = computed(() => [...all.value].sort((a, b) => t(b.added_at) - t(a.added_at)).slice(0, 24));
const topRated = computed(() => all.value.filter((m) => (m.rating ?? 0) >= 7.5).sort((a, b) => (b.rating ?? 0) - (a.rating ?? 0)).slice(0, 24));
const byKind = (kind: "movie" | "series") =>
  [...all.value.filter((m) => m.kind === kind)].sort((a, b) => Number(unwatched(b)) - Number(unwatched(a)) || t(b.added_at) - t(a.added_at)).slice(0, 30);
const movies = computed(() => byKind("movie"));
const series = computed(() => byKind("series"));
/** The genre watched most, with what is still unwatched in it. */
const favouriteGenre = computed(() => {
  const counts = new Map<string, number>();
  for (const m of all.value) if (m.watched_count > 0) for (const g of (m.genres ?? "").split(", ").filter(Boolean)) counts.set(g, (counts.get(g) ?? 0) + 1);
  const top = [...counts.entries()].sort((a, b) => b[1] - a[1])[0]?.[0];
  if (!top) return null;
  const list = all.value.filter((m) => unwatched(m) && (m.genres ?? "").split(", ").includes(top)).slice(0, 24);
  return list.length >= 3 ? { genre: top, list } : null;
});

function sub(it: ContinueItem) {
  if (it.kind === "series") return `${episodeCode(it.episode)} · ${it.episode.position_secs > 0 ? hms(it.episode.position_secs) : "next up"}`;
  return it.episode.position_secs > 0 ? `paused at ${hms(it.episode.position_secs)}` : "next up";
}
const itemSub = (m: MediaItem) => [m.year, m.kind === "movie" ? "Movie" : `${m.episode_count} ep`].filter(Boolean).join(" · ");

async function load() {
  const [c, media, cols] = await Promise.all([api.continueWatching(), api.listMedia(), api.listTags("collection")]);
  items.value = c;
  all.value = media;
  collections.value = cols.filter((x) => x.item_count > 0 && x.watched_count < x.item_count).slice(0, 12);
  loading.value = false;
  await refreshHero();
}

async function play(item: ContinueItem) {
  try {
    const tracked = await api.playEpisode(item.episode.id);
    if (!tracked) toast.warning("Opened with the default app. Choose a player in Settings to track progress.");
  } catch (e) {
    toast.error(String(e));
  }
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

async function dismiss(item: ContinueItem) {
  await api.hideFromHome(item.episode.id);
  items.value = items.value.filter((i) => i.episode.id !== item.episode.id);
  if (hero.value?.cont?.episode.id === item.episode.id) await chooseHero();
}

onMounted(load);
defineExpose({ reload: load });
</script>

<template>
  <div>
    <!-- Billboard -->
    <section v-if="hero" class="relative h-[min(82vh,780px)] min-h-[540px] overflow-hidden">
      <img v-if="heroBackdrop" :key="heroBackdrop" :src="heroBackdrop" alt="" draggable="false"
           class="absolute inset-0 h-full w-full object-cover object-[center_22%] animate-in fade-in duration-700" />
      <img v-else-if="posterSrc(hero.item)" :src="posterSrc(hero.item)!" alt="" draggable="false"
           class="absolute inset-0 h-full w-full scale-125 object-cover opacity-50 blur-3xl" />
      <div class="hero-fade absolute inset-0"></div>

      <div class="relative z-10 mx-auto flex h-full max-w-[1920px] items-end px-10 pb-28">
        <div :key="hero.item.id" class="max-w-2xl animate-in fade-in slide-in-from-bottom-4 duration-500">
          <div class="text-brand mb-3 text-xs font-extrabold uppercase tracking-[0.24em]">{{ hero.label }}</div>
          <h1 class="text-[clamp(2.6rem,5.2vw,4.75rem)] font-black leading-[0.95] tracking-[-0.04em]">{{ heroDetails?.title || hero.item.title }}</h1>
          <div class="mt-5 flex flex-wrap items-center gap-x-2.5 gap-y-2 text-sm font-medium text-foreground/80">
            <span v-if="heroRating" class="bg-brand flex items-center gap-1 rounded-full px-2.5 py-0.5 text-xs font-extrabold"><Star class="size-3 fill-current" />{{ heroRating.toFixed(1) }}</span>
            <template v-for="(p, i) in heroMeta" :key="p">
              <span v-if="i > 0 || heroRating" class="size-1 rounded-full bg-foreground/35"></span>
              <span>{{ p }}</span>
            </template>
          </div>
          <p v-if="heroOverview" class="mt-4 line-clamp-3 max-w-xl text-[15px] leading-7 text-foreground/75">{{ heroOverview }}</p>
          <div v-if="heroResume" class="mt-5 max-w-md">
            <div v-if="heroResume.label" class="mb-1.5 truncate text-sm font-semibold">{{ heroResume.label }}</div>
            <div class="flex items-center gap-3 text-xs font-medium text-foreground/65">
              <div class="h-1.5 flex-1 overflow-hidden rounded-full bg-foreground/15"><div class="bg-brand h-full rounded-full" :style="{ width: heroResume.pct + '%' }"></div></div>
              {{ heroResume.left }}
            </div>
          </div>
          <div class="mt-8 flex items-center gap-3">
            <Button variant="brand" size="xl" @click="playHero"><Play class="fill-current" /> {{ hero.cont ? "Resume" : "Play" }}</Button>
            <Button variant="soft" size="xl" @click="emit('open', hero.item.id)"><Info /> More info</Button>
            <Button variant="soft" size="icon-lg" class="size-12 rounded-full" title="Surprise me" @click="chooseHero(true)"><Shuffle class="size-5" /></Button>
          </div>
        </div>
      </div>
    </section>

    <!-- Empty library -->
    <section v-else-if="!loading" class="relative flex h-[min(80vh,720px)] min-h-[520px] items-center overflow-hidden">
      <div class="mx-auto max-w-[1920px] px-10">
        <div class="text-brand mb-3 text-xs font-extrabold uppercase tracking-[0.24em]">Welcome to Vortex</div>
        <h1 class="max-w-3xl text-[clamp(2.6rem,5vw,4.5rem)] font-black leading-[0.95] tracking-[-0.04em]">Your films and series, beautifully in one place.</h1>
        <p class="mt-5 max-w-xl text-[15px] leading-7 text-foreground/70">Add a folder or a whole drive. Vortex finds every movie and episode, fetches posters from TMDb and remembers exactly where you stopped.</p>
        <div class="mt-8 flex gap-3">
          <Button variant="brand" size="xl" @click="emit('go', 'settings')"><FolderPlus /> Add a folder</Button>
        </div>
      </div>
    </section>
    <div v-else class="px-10 pt-24"><Skeleton class="h-[60vh] rounded-3xl" /></div>

    <!-- Rows -->
    <div ref="root" class="relative z-10 space-y-6 pb-16" :class="{ '-mt-20': hero }">
      <MediaRow v-if="items.length" title="Continue watching" :count="items.length">
        <CarouselItem v-for="it in items" :key="it.episode.id" class="basis-auto pl-5 first:pl-10 last:pr-10">
          <MediaCard class="w-[172px]"
                   :title="it.title" :poster="posterSrc(it)" :subtitle="sub(it)" :meta="relativeTime(it.episode.last_watched)"
                   :progress="it.episode.duration_secs ? it.episode.position_secs / it.episode.duration_secs : null"
                   dismissable playable :unavailable="!it.episode.available"
                   @open="emit('open', it.episode.media_item_id)" @play="play(it)" @dismiss="dismiss(it)" />
        </CarouselItem>
      </MediaRow>

      <MediaRow v-if="recent.length" title="Recently added">
        <CarouselItem v-for="m in recent" :key="m.id" class="basis-auto pl-5 first:pl-10 last:pr-10">
          <MediaCard class="w-[172px]" :title="m.title" :poster="posterSrc(m)"
                   :subtitle="itemSub(m)" :rating="m.rating" :done="m.episode_count > 0 && m.watched_count >= m.episode_count" playable
                   @open="emit('open', m.id)" @play="playItem(m)" />
        </CarouselItem>
      </MediaRow>

      <MediaRow v-if="topRated.length >= 3" title="Top rated in your library">
        <CarouselItem v-for="m in topRated" :key="m.id" class="basis-auto pl-5 first:pl-10 last:pr-10">
          <MediaCard class="w-[172px]" :title="m.title" :poster="posterSrc(m)"
                   :subtitle="itemSub(m)" :rating="m.rating" :done="m.episode_count > 0 && m.watched_count >= m.episode_count" playable
                   @open="emit('open', m.id)" @play="playItem(m)" />
        </CarouselItem>
      </MediaRow>

      <MediaRow v-if="favouriteGenre" :title="`More ${favouriteGenre.genre}`">
        <CarouselItem v-for="m in favouriteGenre.list" :key="m.id" class="basis-auto pl-5 first:pl-10 last:pr-10">
          <MediaCard class="w-[172px]" :title="m.title" :poster="posterSrc(m)"
                   :subtitle="itemSub(m)" :rating="m.rating" playable @open="emit('open', m.id)" @play="playItem(m)" />
        </CarouselItem>
      </MediaRow>

      <MediaRow v-if="movies.length" title="Movies" :count="all.filter((m) => m.kind === 'movie').length" see-all @see-all="emit('go', 'movies')">
        <CarouselItem v-for="m in movies" :key="m.id" class="basis-auto pl-5 first:pl-10 last:pr-10">
          <MediaCard class="w-[172px]" :title="m.title" :poster="posterSrc(m)"
                   :subtitle="itemSub(m)" :rating="m.rating" :done="m.episode_count > 0 && m.watched_count >= m.episode_count" playable
                   @open="emit('open', m.id)" @play="playItem(m)" />
        </CarouselItem>
      </MediaRow>

      <MediaRow v-if="series.length" title="Series" :count="all.filter((m) => m.kind === 'series').length" see-all @see-all="emit('go', 'series')">
        <CarouselItem v-for="m in series" :key="m.id" class="basis-auto pl-5 first:pl-10 last:pr-10">
          <MediaCard class="w-[172px]" :title="m.title" :poster="posterSrc(m)"
                   :subtitle="itemSub(m)" :rating="m.rating" :done="m.episode_count > 0 && m.watched_count >= m.episode_count"
                   :progress="m.episode_count ? m.watched_count / m.episode_count : null" playable
                   @open="emit('open', m.id)" @play="playItem(m)" />
        </CarouselItem>
      </MediaRow>

      <MediaRow v-if="collections.length" title="Collections in progress" see-all @see-all="emit('go', 'collections')">
        <CarouselItem v-for="c in collections" :key="c.id" class="basis-auto pl-5 first:pl-10 last:pr-10">
          <MediaCard class="w-[172px]" :title="c.name" :poster="collectionPoster(c)"
                   :subtitle="`${c.watched_count} of ${c.item_count} watched`" :progress="c.watched_count / c.item_count"
                   @open="emit('go', 'collections')" />
        </CarouselItem>
      </MediaRow>

      <div v-if="!loading && all.length && !items.length && !recent.length" class="px-10 text-sm text-muted-foreground"><Film class="inline size-4" /> Nothing to show yet.</div>
    </div>
  </div>
</template>
