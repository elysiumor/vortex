<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { toast } from "vue-sonner";
import { Shuffle, Layers, Clock, PlusCircle, Play, Film } from "@lucide/vue";
import { api, collectionPoster, posterSrc, type ContinueItem, type MediaItem, type Tag } from "../lib/api";
import { episodeCode, hms, relativeTime } from "../lib/format";
import { useGridKeys } from "../composables/useGridKeys";
import MediaCard from "./MediaCard.vue";
import EmptyState from "./EmptyState.vue";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";

const emit = defineEmits<{ open: [id: number]; go: [page: "collections" | "movies" | "series"] }>();
const items = ref<ContinueItem[]>([]);
const recent = ref<MediaItem[]>([]);
const collections = ref<Tag[]>([]);
const loading = ref(true);
const pick = ref<MediaItem | null>(null);
const root = ref<HTMLElement>();
useGridKeys(root);

async function load() {
  const [c, all, cols] = await Promise.all([api.continueWatching(), api.listMedia(), api.listTags("collection")]);
  items.value = c;
  const t = (s: string | null) => (s ? new Date(s.replace(" ", "T") + "Z").getTime() : 0);
  recent.value = [...all].sort((a, b) => t(b.added_at) - t(a.added_at)).slice(0, 12);
  collections.value = cols.filter((x) => x.item_count > 0 && x.watched_count < x.item_count).slice(0, 8);
  loading.value = false;
}

const unwatched = computed(() => recent.value); // placeholder to keep template simple

async function randomPick() {
  const all = await api.listMedia();
  const pool = all.filter((m) => m.watched_count < m.episode_count);
  pick.value = pool.length ? pool[Math.floor(Math.random() * pool.length)] : null;
  if (!pick.value) toast("Everything is watched. Impressive.");
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
}

async function clearAll() {
  await api.hideAllFromHome();
  items.value = [];
  toast("Continue watching cleared. Progress is kept; play a title to bring it back.");
}

function sub(it: ContinueItem) {
  const parts = [];
  if (it.kind === "series") parts.push(episodeCode(it.episode));
  parts.push(it.episode.position_secs > 0 ? `paused at ${hms(it.episode.position_secs)}` : "next up");
  return parts.join(" · ");
}

onMounted(load);
defineExpose({ reload: load });
</script>

<template>
  <div ref="root" class="space-y-10">
    <section>
      <div class="mb-4 flex items-center gap-3">
        <h1 class="text-2xl font-semibold tracking-tight">Continue watching</h1>
        <div class="flex-1"></div>
        <Button variant="ghost" size="sm" v-if="items.length" @click="clearAll">Clear</Button>
        <Button variant="outline" size="sm" @click="randomPick"><Shuffle /> Surprise me</Button>
      </div>

      <div v-if="pick" class="mb-5 flex items-center gap-4 rounded-xl border bg-card p-3 animate-in fade-in slide-in-from-top-2">
        <img v-if="posterSrc(pick)" :src="posterSrc(pick)!" class="h-24 w-16 rounded-md object-cover" :alt="pick.title" />
        <div class="min-w-0 flex-1">
          <div class="text-xs text-muted-foreground">Tonight's random pick</div>
          <div class="truncate text-lg font-medium">{{ pick.title }} <span class="text-muted-foreground" v-if="pick.year">({{ pick.year }})</span></div>
          <div class="truncate text-sm text-muted-foreground">{{ pick.genres || (pick.kind === "movie" ? "Movie" : "Series") }}</div>
        </div>
        <Button variant="outline" size="sm" @click="randomPick"><Shuffle /> Again</Button>
        <Button size="sm" @click="emit('open', pick.id)">Details</Button>
        <Button size="sm" @click="playItem(pick)"><Play class="fill-current" /> Play</Button>
      </div>

      <div v-if="loading" class="grid grid-cols-[repeat(auto-fill,minmax(160px,1fr))] gap-4">
        <Skeleton v-for="i in 6" :key="i" class="aspect-[2/3] rounded-xl" />
      </div>
      <EmptyState v-else-if="items.length === 0" title="Nothing in progress" hint="Open a movie or series and press Play. Whatever you pause shows up here, ready to resume.">
        <template #icon><Clock class="size-6" /></template>
        <div class="flex gap-2">
          <Button variant="outline" size="sm" @click="emit('go', 'movies')"><Film /> Browse movies</Button>
          <Button variant="outline" size="sm" @click="emit('go', 'series')">Browse series</Button>
        </div>
      </EmptyState>
      <div v-else class="grid grid-cols-[repeat(auto-fill,minmax(160px,1fr))] gap-4">
        <MediaCard v-for="it in items" :key="it.episode.id"
                   :title="it.title" :poster="posterSrc(it)" :subtitle="sub(it)" :meta="relativeTime(it.episode.last_watched)"
                   :progress="it.episode.duration_secs ? it.episode.position_secs / it.episode.duration_secs : null"
                   dismissable playable :unavailable="!it.episode.available"
                   @open="emit('open', it.episode.media_item_id)" @play="play(it)" @dismiss="dismiss(it)" />
      </div>
    </section>

    <section v-if="collections.length">
      <div class="mb-4 flex items-center gap-3">
        <h2 class="text-lg font-semibold tracking-tight">Next up in collections</h2>
        <Button variant="ghost" size="sm" @click="emit('go', 'collections')"><Layers /> All collections</Button>
      </div>
      <div class="grid grid-cols-[repeat(auto-fill,minmax(160px,1fr))] gap-4">
        <MediaCard v-for="c in collections" :key="c.id" :title="c.name" :poster="collectionPoster(c)"
                   :subtitle="`${c.watched_count} of ${c.item_count} watched`" :progress="c.watched_count / c.item_count"
                   @open="emit('go', 'collections')" />
      </div>
    </section>

    <section v-if="unwatched.length">
      <div class="mb-4 flex items-center gap-3">
        <h2 class="text-lg font-semibold tracking-tight">Recently added</h2>
        <PlusCircle class="size-4 text-muted-foreground" />
      </div>
      <div class="grid grid-cols-[repeat(auto-fill,minmax(160px,1fr))] gap-4">
        <MediaCard v-for="m in recent" :key="m.id" :title="m.title" :poster="posterSrc(m)"
                   :subtitle="[m.year, m.kind === 'movie' ? 'Movie' : `${m.episode_count} episodes`].filter(Boolean).join(' · ')"
                   :meta="m.added_at ? `added ${relativeTime(m.added_at)}` : undefined"
                   :done="m.episode_count > 0 && m.watched_count >= m.episode_count" playable
                   @open="emit('open', m.id)" @play="playItem(m)" />
      </div>
    </section>
  </div>
</template>
