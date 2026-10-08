<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { toast } from "vue-sonner";
import { ArrowRight, Folder, FileVideo, FileText } from "@lucide/vue";
import { api, type RenameMove, type RenamePlan } from "../lib/api";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";

/**
 * Rename movies to their TMDb names. Shows exactly what would change and
 * changes nothing until "Rename" is pressed; the last rename can be undone.
 * `ids` null means every movie in the library.
 */
const props = defineProps<{ ids: number[] | null }>();
const open = defineModel<boolean>("open", { required: true });
const emit = defineEmits<{ done: [] }>();

const plans = ref<RenamePlan[]>([]);
const loading = ref(false);
const applying = ref(false);
const chosen = ref(new Set<number>());

const toRename = computed(() => plans.value.filter((p) => !p.skipped && p.moves.length > 0));
const alreadyNamed = computed(() => plans.value.filter((p) => !p.skipped && p.moves.length === 0));
const skipped = computed(() => plans.value.filter((p) => p.skipped));

watch(open, async (isOpen) => {
  if (!isOpen) return;
  plans.value = [];
  expanded.value = new Set();
  loading.value = true;
  try {
    plans.value = await api.renamePreview(props.ids ?? undefined);
    chosen.value = new Set(toRename.value.map((p) => p.media_item_id));
  } catch (e) {
    toast.error(String(e));
    open.value = false;
  } finally {
    loading.value = false;
  }
});

function toggle(id: number) {
  const s = new Set(chosen.value);
  if (s.has(id)) s.delete(id); else s.add(id);
  chosen.value = s;
}

// A series can mean hundreds of changes; show the folders and a few files.
const MOVES_SHOWN = 6;
const expanded = ref(new Set<number>());
function expand(id: number) { expanded.value = new Set(expanded.value).add(id); }
function shownMoves(p: RenamePlan): RenameMove[] {
  if (expanded.value.has(p.media_item_id) || p.moves.length <= MOVES_SHOWN) return p.moves;
  const folders = p.moves.filter((m) => m.folder);
  return [...folders, ...p.moves.filter((m) => !m.folder).slice(0, Math.max(1, MOVES_SHOWN - folders.length))];
}

const nameOf = (p: string) => p.split(/[\\/]/).pop() ?? p;
const iconFor = (m: RenameMove) =>
  m.folder ? Folder : /\.(mkv|mp4|avi|mov|wmv|flv|webm|m4v|ts|mpg|mpeg|m2ts|vob|3gp|ogv)$/i.test(m.from) ? FileVideo : FileText;

async function apply() {
  const ids = [...chosen.value];
  if (ids.length === 0) return;
  applying.value = true;
  try {
    const r = await api.renameApply(ids);
    open.value = false;
    emit("done");
    if (r.renamed > 0) {
      toast.success(`Renamed ${r.renamed} title${r.renamed === 1 ? "" : "s"}`, {
        action: { label: "Undo", onClick: () => undo() },
      });
    }
    for (const [title, why] of r.failed) toast.error(`${title}: ${why}`);
  } catch (e) {
    toast.error(String(e));
  } finally {
    applying.value = false;
  }
}

async function undo() {
  try {
    await api.renameUndo();
    emit("done");
    toast("Rename undone");
  } catch (e) {
    toast.error(String(e));
  }
}
</script>

<template>
  <Dialog v-model:open="open">
    <DialogContent class="sm:max-w-3xl grid-cols-[minmax(0,1fr)]">
      <DialogHeader>
        <DialogTitle>Rename to TMDb names</DialogTitle>
        <DialogDescription>
          Movies become <span class="font-medium text-foreground">Title (Year)</span>, episodes <span class="font-medium text-foreground">Show (Year) - S01E01 - Episode</span>, with subtitles alongside. Anime numbered straight through is placed on TMDb's seasons. Folders follow when they hold only that title, and season folders become Season 01. Nothing changes until you press Rename, and the last rename can be undone.
        </DialogDescription>
      </DialogHeader>

      <p v-if="loading" class="py-6 text-center text-sm text-muted-foreground">Checking names…</p>
      <div v-else class="max-h-[60vh] space-y-3 overflow-y-auto pr-1">
        <p v-if="toRename.length === 0" class="text-sm text-muted-foreground">
          {{ alreadyNamed.length ? "Everything here is already named after TMDb." : "Nothing can be renamed." }}
        </p>
        <label v-for="p in toRename" :key="p.media_item_id" class="flex cursor-pointer gap-3 rounded-lg border p-3 hover:bg-accent/40">
          <Checkbox class="mt-0.5" :model-value="chosen.has(p.media_item_id)" @update:model-value="toggle(p.media_item_id)" @click.stop />
          <div class="min-w-0 flex-1 space-y-1">
            <div class="font-medium">{{ p.target }} <span v-if="p.moves.length > 1" class="text-xs font-normal text-muted-foreground">· {{ p.moves.length }} changes</span></div>
            <div v-for="m in shownMoves(p)" :key="m.from" class="flex min-w-0 items-center gap-1.5 text-xs text-muted-foreground">
              <component :is="iconFor(m)" class="size-3.5 shrink-0" />
              <span class="truncate" :title="m.from">{{ nameOf(m.from) }}</span>
              <ArrowRight class="size-3 shrink-0" />
              <span class="truncate text-foreground" :title="m.to">{{ nameOf(m.to) }}</span>
            </div>
            <button v-if="p.moves.length > MOVES_SHOWN && !expanded.has(p.media_item_id)" type="button" class="text-xs text-primary hover:underline" @click.prevent="expand(p.media_item_id)">Show all {{ p.moves.length }}</button>
            <div v-for="n in p.notes" :key="n" class="text-xs text-warning">{{ n }}</div>
          </div>
        </label>

        <details v-if="skipped.length" class="rounded-lg border p-3 text-sm">
          <summary class="cursor-pointer text-muted-foreground">Skipped: {{ skipped.length }}</summary>
          <ul class="mt-2 space-y-1">
            <li v-for="p in skipped" :key="p.media_item_id" class="text-xs"><span class="font-medium">{{ p.title || `#${p.media_item_id}` }}</span> <span class="text-muted-foreground">· {{ p.skipped }}</span></li>
          </ul>
        </details>
        <p v-if="alreadyNamed.length && toRename.length" class="text-xs text-muted-foreground">{{ alreadyNamed.length }} already named after TMDb.</p>
      </div>

      <DialogFooter>
        <Button variant="outline" @click="open = false">Cancel</Button>
        <Button :disabled="loading || applying || chosen.size === 0" @click="apply">
          {{ applying ? "Renaming…" : `Rename ${chosen.size} title${chosen.size === 1 ? "" : "s"}` }}
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
