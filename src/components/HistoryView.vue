<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { toast } from "vue-sonner";
import { Play, X, History as HistoryIcon, Trash2 } from "@lucide/vue";
import { api, type HistoryEntry, type HistoryStats } from "../lib/api";
import { episodeCode, episodeTitle, hms } from "../lib/format";
import EmptyState from "./EmptyState.vue";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { AlertDialog, AlertDialogAction, AlertDialogCancel, AlertDialogContent, AlertDialogDescription, AlertDialogFooter, AlertDialogHeader, AlertDialogTitle } from "@/components/ui/alert-dialog";

const emit = defineEmits<{ open: [id: number] }>();
const entries = ref<HistoryEntry[]>([]);
const stats = ref<HistoryStats | null>(null);
const confirmClear = ref(false);

async function load() { [entries.value, stats.value] = await Promise.all([api.listHistory(), api.historyStats()]); }

function dayOf(at: string): string {
  const d = new Date(at.replace(" ", "T") + "Z"), today = new Date(), y = new Date(today);
  y.setDate(today.getDate() - 1);
  if (d.toDateString() === today.toDateString()) return "Today";
  if (d.toDateString() === y.toDateString()) return "Yesterday";
  return d.toLocaleDateString(undefined, { weekday: "short", day: "numeric", month: "short", year: d.getFullYear() !== today.getFullYear() ? "numeric" : undefined });
}
const timeOf = (at: string) => new Date(at.replace(" ", "T") + "Z").toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" });
const groups = computed(() => {
  const map = new Map<string, HistoryEntry[]>();
  for (const e of entries.value) map.set(dayOf(e.at), [...(map.get(dayOf(e.at)) ?? []), e]);
  return [...map.entries()];
});
async function remove(e: HistoryEntry) { await api.deleteHistory(e.id); await load(); }
async function clearAll() { await api.clearHistory(); confirmClear.value = false; await load(); }
async function play(e: HistoryEntry) { try { await api.playEpisode(e.episode.id); } catch (err) { toast.error(String(err)); } }

const tiles = computed(() => stats.value ? [
  [stats.value.sessions_30d, "sessions, last 30 days"], [stats.value.completed_year, "finished this year"],
  [stats.value.hours_year, "hours this year"], [stats.value.total_sessions, "all time"],
] : []);

onMounted(load);
defineExpose({ reload: load });
</script>

<template>
  <div class="space-y-6">
    <div class="flex items-center gap-3">
      <h1 class="flex-1 text-2xl font-semibold tracking-tight">History</h1>
      <Button variant="outline" size="sm" v-if="entries.length" @click="confirmClear = true"><Trash2 /> Clear history</Button>
    </div>
    <div class="grid grid-cols-2 gap-3 md:grid-cols-4" v-if="stats">
      <Card v-for="[n, l] in tiles" :key="String(l)"><CardContent class="p-4"><div class="text-2xl font-semibold tabular-nums">{{ n }}</div><div class="text-xs text-muted-foreground">{{ l }}</div></CardContent></Card>
    </div>

    <EmptyState v-if="entries.length === 0" title="Nothing yet" hint="Every play session and manual watched mark is logged here."><template #icon><HistoryIcon class="size-6" /></template></EmptyState>

    <section v-for="[day, list] in groups" :key="day">
      <h2 class="mb-2 text-sm font-medium uppercase tracking-wider text-muted-foreground">{{ day }}</h2>
      <div class="space-y-1.5">
        <div v-for="e in list" :key="e.id" class="grid grid-cols-[56px_1fr_auto] items-center gap-3 rounded-lg border bg-card px-3 py-2">
          <div class="text-sm tabular-nums text-muted-foreground">{{ timeOf(e.at) }}</div>
          <div class="min-w-0">
            <div class="truncate"><button class="font-medium hover:text-primary" @click="emit('open', e.media_item_id)">{{ e.item_title }}</button><span v-if="e.kind === 'series'"> · {{ episodeCode(e.episode) }} {{ episodeTitle(e.episode) }}</span></div>
            <div class="text-xs text-muted-foreground">
              <span v-if="e.completed" class="text-success">Finished</span><span v-else>Stopped at {{ hms(e.position_secs) }}</span>
              <span v-if="e.source === 'manual'"> · marked by hand</span><span v-else-if="!e.exact"> · estimated</span>
            </div>
          </div>
          <div class="flex items-center gap-1">
            <Button size="icon-sm" variant="ghost" title="Remove this entry" @click="remove(e)"><X /></Button>
            <Button size="sm" :disabled="!e.episode.available" @click="play(e)"><Play class="fill-current" /> Play</Button>
          </div>
        </div>
      </div>
    </section>

    <AlertDialog v-model:open="confirmClear">
      <AlertDialogContent>
        <AlertDialogHeader><AlertDialogTitle>Clear all history?</AlertDialogTitle><AlertDialogDescription>This removes the log only. Watched marks and paused positions are kept.</AlertDialogDescription></AlertDialogHeader>
        <AlertDialogFooter><AlertDialogCancel>Cancel</AlertDialogCancel><AlertDialogAction @click="clearAll">Clear</AlertDialogAction></AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  </div>
</template>
