<script setup lang="ts">
import { ref, watch } from "vue";
import { toast } from "vue-sonner";
import { Play, SearchX } from "@lucide/vue";
import { api, posterSrc, type SearchResults } from "../lib/api";
import { episodeCode, episodeTitle, hms } from "../lib/format";
import { useGridKeys } from "../composables/useGridKeys";
import MediaCard from "./MediaCard.vue";
import EmptyState from "./EmptyState.vue";
import { Button } from "@/components/ui/button";

const props = defineProps<{ query: string }>();
const emit = defineEmits<{ open: [id: number] }>();
const results = ref<SearchResults>({ items: [], episodes: [] });
const busy = ref(false);
const root = ref<HTMLElement>();
useGridKeys(root);
let timer: number | undefined;

watch(() => props.query, (q) => {
  clearTimeout(timer);
  if (!q.trim()) { results.value = { items: [], episodes: [] }; return; }
  busy.value = true;
  timer = window.setTimeout(async () => { try { results.value = await api.search(q); } finally { busy.value = false; } }, 150);
}, { immediate: true });

async function play(id: number) { try { await api.playEpisode(id); } catch (e) { toast.error(String(e)); } }
</script>

<template>
  <div ref="root" class="space-y-8">
    <h1 class="text-2xl font-semibold tracking-tight">Search <span class="font-normal text-muted-foreground">{{ query }}</span></h1>
    <EmptyState v-if="!busy && results.items.length === 0 && results.episodes.length === 0" title="No matches" hint="Titles, episode names and file names are searched."><template #icon><SearchX class="size-6" /></template></EmptyState>

    <section v-if="results.items.length">
      <h2 class="mb-3 text-sm font-medium uppercase tracking-wider text-muted-foreground">Titles</h2>
      <div class="grid grid-cols-[repeat(auto-fill,minmax(160px,1fr))] gap-4">
        <MediaCard v-for="m in results.items" :key="m.id" :title="m.title" :poster="posterSrc(m)"
                   :subtitle="[m.kind === 'movie' ? 'Movie' : 'Series', m.year].filter(Boolean).join(' · ')" @open="emit('open', m.id)" />
      </div>
    </section>

    <section v-if="results.episodes.length">
      <h2 class="mb-3 text-sm font-medium uppercase tracking-wider text-muted-foreground">Episodes</h2>
      <div class="space-y-1.5">
        <div v-for="h in results.episodes" :key="h.episode.id" class="grid grid-cols-[72px_1fr_auto] items-center gap-3 rounded-lg border bg-card px-3 py-2" :class="{ 'opacity-60': !h.episode.available }">
          <div class="text-sm font-semibold" :class="h.episode.completed ? 'text-success' : 'text-primary'">{{ episodeCode(h.episode) || "▶" }}</div>
          <div class="min-w-0">
            <div class="truncate"><button class="font-medium hover:text-primary" @click="emit('open', h.episode.media_item_id)">{{ h.item_title }}</button><span v-if="h.kind === 'series'"> · {{ episodeTitle(h.episode) }}</span></div>
            <div class="truncate text-xs text-muted-foreground">{{ h.episode.file_name }}<span v-if="h.episode.position_secs > 0 && !h.episode.completed"> · paused at {{ hms(h.episode.position_secs) }}</span></div>
          </div>
          <Button size="sm" :disabled="!h.episode.available" @click="play(h.episode.id)"><Play class="fill-current" /> Play</Button>
        </div>
      </div>
    </section>
  </div>
</template>
