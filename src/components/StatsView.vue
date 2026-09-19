<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { BarChart3 } from "@lucide/vue";
import { api, type Stats } from "../lib/api";
import { relativeTime } from "../lib/format";
import EmptyState from "./EmptyState.vue";
import { Card, CardContent } from "@/components/ui/card";
import { Progress } from "@/components/ui/progress";
import { Skeleton } from "@/components/ui/skeleton";

const emit = defineEmits<{ open: [id: number] }>();
const s = ref<Stats | null>(null);

async function load() {
  s.value = await api.stats();
}

const tb = (b: number) => (b >= 1e12 ? `${(b / 1e12).toFixed(2)} TB` : `${(b / 1073741824).toFixed(0)} GB`);
const maxMonth = computed(() => Math.max(1, ...(s.value?.hours_by_month.map((m) => m[1]) ?? [1])));
const maxGenre = computed(() => Math.max(1, ...(s.value?.top_genres.map((g) => g[1]) ?? [1])));
const maxStorage = computed(() => Math.max(1, ...(s.value?.storage_by_category.map((c) => c[1]) ?? [1])));
const monthLabel = (ym: string) => new Date(ym + "-02").toLocaleDateString(undefined, { month: "short", year: "2-digit" });

const tiles = computed(() => s.value ? [
  { n: s.value.hours_total, l: "hours watched" },
  { n: `${s.value.watched_movies} / ${s.value.movies}`, l: "movies watched" },
  { n: `${s.value.watched_episodes} / ${s.value.episodes}`, l: "episodes watched" },
  { n: s.value.series, l: "series in library" },
  { n: tb(s.value.total_bytes), l: "on disk" },
] : []);

onMounted(load);
defineExpose({ reload: load });
</script>

<template>
  <div class="space-y-8">
    <h1 class="text-2xl font-semibold tracking-tight">Statistics</h1>
    <div v-if="!s" class="grid grid-cols-5 gap-3"><Skeleton v-for="i in 5" :key="i" class="h-20 rounded-xl" /></div>
    <template v-else>
      <div class="grid grid-cols-2 gap-3 md:grid-cols-5">
        <Card v-for="t in tiles" :key="t.l"><CardContent class="p-4">
          <div class="text-2xl font-semibold tabular-nums">{{ t.n }}</div>
          <div class="text-xs text-muted-foreground">{{ t.l }}</div>
        </CardContent></Card>
      </div>

      <div class="grid gap-6 lg:grid-cols-2">
        <Card><CardContent class="p-5">
          <h2 class="mb-4 text-sm font-medium text-muted-foreground">Hours per month</h2>
          <EmptyState v-if="s.hours_by_month.length === 0" title="No sessions yet" hint="Play something and come back."><template #icon><BarChart3 class="size-6" /></template></EmptyState>
          <div v-else class="flex h-40 items-end gap-2">
            <div v-for="[ym, h] in s.hours_by_month" :key="ym" class="flex flex-1 flex-col items-center gap-1">
              <div class="text-[11px] tabular-nums text-muted-foreground">{{ h }}</div>
              <div class="w-full rounded-t-md bg-primary/80 transition-all" :style="{ height: (100 * h / maxMonth) + '%' }" :title="`${h} h`"></div>
              <div class="text-[10px] text-muted-foreground">{{ monthLabel(ym) }}</div>
            </div>
          </div>
        </CardContent></Card>

        <Card><CardContent class="p-5">
          <h2 class="mb-4 text-sm font-medium text-muted-foreground">Genres you finish most</h2>
          <div v-if="s.top_genres.length === 0" class="text-sm text-muted-foreground">Match titles to TMDB and finish a few.</div>
          <div v-else class="space-y-2.5">
            <div v-for="[g, n] in s.top_genres" :key="g">
              <div class="mb-1 flex justify-between text-sm"><span>{{ g }}</span><span class="tabular-nums text-muted-foreground">{{ n }}</span></div>
              <Progress :model-value="100 * n / maxGenre" class="h-1.5" />
            </div>
          </div>
        </CardContent></Card>

        <Card><CardContent class="p-5">
          <h2 class="mb-4 text-sm font-medium text-muted-foreground">Storage by category</h2>
          <div class="space-y-2.5">
            <div v-for="[c, b] in s.storage_by_category" :key="c">
              <div class="mb-1 flex justify-between text-sm"><span>{{ c }}</span><span class="tabular-nums text-muted-foreground">{{ tb(b) }}</span></div>
              <Progress :model-value="100 * b / maxStorage" class="h-1.5" />
            </div>
          </div>
        </CardContent></Card>

        <Card><CardContent class="p-5">
          <h2 class="mb-4 text-sm font-medium text-muted-foreground">Series in progress</h2>
          <div v-if="s.in_progress.length === 0" class="text-sm text-muted-foreground">Nothing half-watched.</div>
          <div v-else class="space-y-2.5">
            <button v-for="[id, title, w, total, lw] in s.in_progress" :key="id" class="block w-full text-left" @click="emit('open', id)">
              <div class="mb-1 flex justify-between text-sm"><span class="truncate hover:text-primary">{{ title }}</span><span class="shrink-0 tabular-nums text-muted-foreground">{{ w }} / {{ total }}<template v-if="lw"> · {{ relativeTime(lw) }}</template></span></div>
              <Progress :model-value="100 * w / total" class="h-1.5" />
            </button>
          </div>
          <template v-if="s.stale.length">
            <h2 class="mb-2 mt-6 text-sm font-medium text-muted-foreground">Gathering dust</h2>
            <div class="flex flex-wrap gap-1.5">
              <button v-for="[id, title, days] in s.stale" :key="id" class="rounded-full border px-2.5 py-1 text-xs hover:bg-accent" @click="emit('open', id)">
                {{ title }} <span class="text-muted-foreground">· {{ days }} d</span>
              </button>
            </div>
          </template>
        </CardContent></Card>
      </div>
    </template>
  </div>
</template>
