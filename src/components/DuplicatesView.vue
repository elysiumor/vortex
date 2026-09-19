<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { toast } from "vue-sonner";
import { FolderOpen, Trash2, CopyCheck } from "@lucide/vue";
import { api, type DuplicateGroup, type Episode } from "../lib/api";
import { dateFromUnix, fileSize, folderOf, hms } from "../lib/format";
import EmptyState from "./EmptyState.vue";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { AlertDialog, AlertDialogAction, AlertDialogCancel, AlertDialogContent, AlertDialogDescription, AlertDialogFooter, AlertDialogHeader, AlertDialogTitle } from "@/components/ui/alert-dialog";

const emit = defineEmits<{ open: [id: number] }>();
const groups = ref<DuplicateGroup[]>([]);
const loading = ref(true);
const pending = ref<{ group: DuplicateGroup; file: Episode } | null>(null);

async function load() { groups.value = await api.findDuplicates(); loading.value = false; }
const wasted = computed(() => groups.value.reduce((sum, g) => sum + g.files.map((f) => f.size).sort((a, b) => b - a).slice(1).reduce((a, b) => a + b, 0), 0));
function label(g: DuplicateGroup) {
  let s = g.item_title;
  if (g.year) s += ` (${g.year})`;
  if (g.kind === "series" && g.season != null && g.episode != null) s += ` · S${String(g.season).padStart(2, "0")}E${String(g.episode).padStart(2, "0")}`;
  return s;
}
const quality = (f: Episode) => /\b(2160p|1080p|720p|480p)\b/i.exec(f.file_name)?.[1] ?? "—";
async function reveal(f: Episode) { try { await api.revealPath(f.path); } catch (e) { toast.error(String(e)); } }
async function confirmTrash() {
  if (!pending.value) return;
  try { await api.trashEpisode(pending.value.file.id); toast.success("Moved to Recycle Bin"); } catch (e) { toast.error(String(e)); }
  pending.value = null;
  await load();
}
onMounted(load);
defineExpose({ reload: load });
</script>

<template>
  <div class="space-y-6">
    <div class="flex items-center gap-3">
      <h1 class="flex-1 text-2xl font-semibold tracking-tight">Duplicates</h1>
      <span class="text-sm text-muted-foreground" v-if="groups.length">{{ groups.length }} groups · {{ fileSize(wasted) }} recoverable</span>
    </div>
    <p v-if="loading" class="text-sm text-muted-foreground">Checking…</p>
    <EmptyState v-else-if="groups.length === 0" title="No duplicates" hint="Every movie and episode exists exactly once."><template #icon><CopyCheck class="size-6" /></template></EmptyState>

    <section v-for="g in groups" :key="`${g.media_item_id}-${g.season}-${g.episode}`">
      <div class="mb-2 font-medium"><button class="hover:text-primary" @click="emit('open', g.media_item_id)">{{ label(g) }}</button> <span class="text-sm text-muted-foreground">· {{ g.files.length }} copies</span></div>
      <div class="space-y-1.5">
        <div v-for="f in g.files" :key="f.id" class="grid grid-cols-[64px_1fr_auto] items-center gap-3 rounded-lg border bg-card px-3 py-2" :class="{ 'opacity-60': !f.available }">
          <Badge variant="outline" class="justify-center">{{ quality(f) }}</Badge>
          <div class="min-w-0">
            <div class="truncate font-medium" :title="f.path">{{ f.file_name }}</div>
            <div class="truncate text-xs text-muted-foreground">{{ folderOf(f.path) }} · {{ fileSize(f.size) }}<span v-if="f.duration_secs"> · {{ hms(f.duration_secs) }}</span> · {{ dateFromUnix(f.modified) }}<span v-if="f.completed || f.position_secs > 0"> · has watch progress</span><span v-if="!f.available"> · not available</span></div>
          </div>
          <div class="flex gap-1">
            <Button size="sm" variant="outline" :disabled="!f.available" @click="reveal(f)"><FolderOpen /> Explorer</Button>
            <Button size="sm" variant="outline" class="text-destructive" :disabled="!f.available" @click="pending = { group: g, file: f }"><Trash2 /> Delete</Button>
          </div>
        </div>
      </div>
    </section>

    <AlertDialog :open="!!pending" @update:open="(v) => !v && (pending = null)">
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>Move this file to the Recycle Bin?</AlertDialogTitle>
          <AlertDialogDescription class="break-all">{{ pending?.file.path }}<br /><span class="mt-2 block">{{ pending ? fileSize(pending.file.size) : "" }}. You can restore it from the Recycle Bin. The other {{ (pending?.group.files.length ?? 1) - 1 }} cop{{ (pending?.group.files.length ?? 1) - 1 === 1 ? "y stays" : "ies stay" }}.</span></AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter><AlertDialogCancel>Cancel</AlertDialogCancel><AlertDialogAction class="bg-destructive text-white hover:bg-destructive/90" @click="confirmTrash">Move to Recycle Bin</AlertDialogAction></AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  </div>
</template>
