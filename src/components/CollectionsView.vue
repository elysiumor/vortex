<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { toast } from "vue-sonner";
import { ArrowLeft, ArrowUp, ArrowDown, X, Play, Plus, Layers, Pencil, Trash2 } from "@lucide/vue";
import { api, collectionPoster, posterSrc, type MediaItem, type Tag } from "../lib/api";
import { useGridKeys } from "../composables/useGridKeys";
import MediaCard from "./MediaCard.vue";
import EmptyState from "./EmptyState.vue";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { AlertDialog, AlertDialogAction, AlertDialogCancel, AlertDialogContent, AlertDialogDescription, AlertDialogFooter, AlertDialogHeader, AlertDialogTitle } from "@/components/ui/alert-dialog";

const emit = defineEmits<{ open: [id: number] }>();
const collections = ref<Tag[]>([]);
const current = ref<Tag | null>(null);
const items = ref<MediaItem[]>([]);
const newName = ref("");
const renaming = ref(false);
const renameValue = ref("");
const confirmDelete = ref(false);
const root = ref<HTMLElement>();
useGridKeys(root);

async function load() {
  collections.value = await api.listTags("collection");
  if (current.value) {
    current.value = collections.value.find((c) => c.id === current.value!.id) ?? null;
    if (current.value) items.value = await api.collectionItems(current.value.id);
  }
}
async function openCollection(c: Tag) { current.value = c; items.value = await api.collectionItems(c.id); }
function back() { current.value = null; items.value = []; }
async function create() {
  const name = newName.value.trim();
  if (!name) return;
  try { const id = await api.createTag("collection", name); newName.value = ""; await load(); const c = collections.value.find((x) => x.id === id); if (c) await openCollection(c); }
  catch (e) { toast.error(String(e)); }
}
async function rename() { if (!current.value) return; try { await api.renameTag(current.value.id, renameValue.value); renaming.value = false; await load(); } catch (e) { toast.error(String(e)); } }
async function remove() { if (!current.value) return; await api.deleteTag(current.value.id); confirmDelete.value = false; back(); await load(); }
async function removeItem(m: MediaItem) { if (!current.value) return; await api.removeFromCollection(current.value.id, m.id); await load(); }
async function move(i: number, dir: -1 | 1) {
  if (!current.value) return;
  const j = i + dir;
  if (j < 0 || j >= items.value.length) return;
  const arr = [...items.value];
  [arr[i], arr[j]] = [arr[j], arr[i]];
  items.value = arr;
  await api.setCollectionOrder(current.value.id, arr.map((m) => m.id));
}
const nextUp = computed(() => items.value.find((m) => m.watched_count < m.episode_count) ?? null);
async function playNext() {
  const m = nextUp.value;
  if (!m) return;
  const eps = (await api.listEpisodes(m.id)).filter((e) => !e.extra);
  const ep = eps.find((e) => !e.completed) ?? eps[0];
  if (!ep) return;
  try { await api.playEpisode(ep.id); } catch (e) { toast.error(String(e)); }
}
watch(current, (c) => { if (c) renameValue.value = c.name; });
onMounted(load);
defineExpose({ reload: load });
</script>

<template>
  <div ref="root" v-if="!current" class="space-y-5">
    <div class="flex items-center gap-3">
      <h1 class="flex-1 text-2xl font-semibold tracking-tight">Collections</h1>
      <Input v-model="newName" placeholder="New collection name" class="h-9 w-56" @keyup.enter="create" />
      <Button :disabled="!newName.trim()" @click="create"><Plus /> Create</Button>
    </div>
    <p class="text-sm text-muted-foreground">Ordered lists of titles. Movie series known to TMDB, such as Alien or Harry Potter, are added automatically in release order.</p>
    <EmptyState v-if="collections.length === 0" title="No collections yet" hint="Create one here or add a title to a new collection from its page."><template #icon><Layers class="size-6" /></template></EmptyState>
    <div v-else class="grid grid-cols-[repeat(auto-fill,minmax(160px,1fr))] gap-4">
      <MediaCard v-for="c in collections" :key="c.id" :title="c.name" :poster="collectionPoster(c)"
                 :subtitle="`${c.item_count} title${c.item_count === 1 ? '' : 's'}${c.tmdb_collection_id ? ' · from TMDB' : ''}`"
                 :badge="`${c.watched_count} / ${c.item_count} watched`" :progress="c.item_count ? c.watched_count / c.item_count : null"
                 :done="c.item_count > 0 && c.watched_count >= c.item_count" @open="openCollection(c)" />
    </div>
  </div>

  <div v-else class="space-y-5">
    <div class="flex flex-wrap items-center gap-2">
      <Button variant="secondary" size="sm" @click="back"><ArrowLeft /> Collections</Button>
      <h1 v-if="!renaming" class="flex-1 text-2xl font-semibold tracking-tight">{{ current.name }}</h1>
      <template v-else>
        <Input v-model="renameValue" class="h-9 flex-1" autofocus @keyup.enter="rename" @keyup.esc="renaming = false" />
        <Button size="sm" @click="rename">Save</Button><Button size="sm" variant="ghost" @click="renaming = false">Cancel</Button>
      </template>
      <Button v-if="!renaming" variant="outline" size="sm" @click="renaming = true"><Pencil /> Rename</Button>
      <Button variant="outline" size="sm" class="text-destructive" @click="confirmDelete = true"><Trash2 /> Delete</Button>
      <Button v-if="nextUp" @click="playNext"><Play class="fill-current" /> Play next: {{ nextUp.title }}</Button>
    </div>
    <p class="text-sm text-muted-foreground" v-if="current.overview">{{ current.overview }}</p>
    <EmptyState v-if="items.length === 0" title="Empty collection" hint='Open a title and use "+ collection" to add it here.' />
    <div class="space-y-1.5">
      <div v-for="(m, i) in items" :key="m.id" class="grid grid-cols-[44px_1fr_auto] items-center gap-3 rounded-lg border bg-card px-3 py-2">
        <img v-if="posterSrc(m)" :src="posterSrc(m)!" :alt="m.title" class="h-16 w-11 cursor-pointer rounded object-cover" @click="emit('open', m.id)" />
        <div v-else class="grid h-16 w-11 cursor-pointer place-items-center rounded bg-muted text-muted-foreground" @click="emit('open', m.id)">{{ m.title.slice(0, 1) }}</div>
        <div class="min-w-0">
          <div class="truncate"><span class="text-muted-foreground">{{ i + 1 }}.</span> <button class="font-medium hover:text-primary" @click="emit('open', m.id)">{{ m.title }}</button> <span class="text-muted-foreground" v-if="m.year">({{ m.year }})</span></div>
          <div class="text-xs text-muted-foreground">{{ m.kind === "movie" ? "Movie" : "Series" }} · <span :class="{ 'text-success': m.watched_count >= m.episode_count && m.episode_count > 0 }">{{ m.kind === "series" ? `${m.watched_count} / ${m.episode_count} watched` : m.watched_count > 0 ? "Watched" : "Unwatched" }}</span></div>
        </div>
        <div class="flex gap-1">
          <Button size="icon-sm" variant="ghost" :disabled="i === 0" @click="move(i, -1)"><ArrowUp /></Button>
          <Button size="icon-sm" variant="ghost" :disabled="i === items.length - 1" @click="move(i, 1)"><ArrowDown /></Button>
          <Button size="icon-sm" variant="ghost" title="Remove from collection" @click="removeItem(m)"><X /></Button>
        </div>
      </div>
    </div>
    <AlertDialog v-model:open="confirmDelete">
      <AlertDialogContent>
        <AlertDialogHeader><AlertDialogTitle>Delete "{{ current.name }}"?</AlertDialogTitle><AlertDialogDescription>Only the list is removed. The titles and your progress stay.</AlertDialogDescription></AlertDialogHeader>
        <AlertDialogFooter><AlertDialogCancel>Cancel</AlertDialogCancel><AlertDialogAction class="bg-destructive text-white hover:bg-destructive/90" @click="remove">Delete</AlertDialogAction></AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  </div>
</template>
