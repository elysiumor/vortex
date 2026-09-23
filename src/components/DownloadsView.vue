<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { toast } from "vue-sonner";
import { open } from "@tauri-apps/plugin-dialog";
import { Download, FileUp, FolderOpen, Play, Pause, Trash2, ShieldCheck, ShieldAlert, ChevronDown, ChevronRight, Settings, Loader2, ArrowDown, ArrowUp, Users, Files, Info, Radio, Activity, Globe, Save } from "@lucide/vue";
import { api, type SessionStatus, type StreamEnded, type TorrentDetail, type TorrentPreview, type TorrentRow, type TorrentStatus } from "../lib/api";
import EmptyState from "./EmptyState.vue";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Progress } from "@/components/ui/progress";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { AlertDialog, AlertDialogAction, AlertDialogCancel, AlertDialogContent, AlertDialogDescription, AlertDialogFooter, AlertDialogHeader, AlertDialogTitle } from "@/components/ui/alert-dialog";

const emit = defineEmits<{ go: [page: "settings"] }>();

const status = ref<TorrentStatus | null>(null);
const rows = ref<TorrentRow[]>([]);
const magnet = ref("");
const inspecting = ref(false);
const expanded = ref<Set<number>>(new Set());

// ---- add flow: inspect → pick files → (unprotected warning) → start ----
const source = ref("");
const preview = ref<TorrentPreview | null>(null);
const picked = ref<Set<number>>(new Set());
const saveIn = ref("");
const createSubfolder = ref(true);
const subfolderName = ref("");
const starting = ref(false);
const warnUnprotected = ref(false);
const removePending = ref<TorrentRow | null>(null);

const destPreview = computed(() => {
  const base = saveIn.value.trim().replace(/[\\/]+$/, "");
  if (!base) return "Choose a folder";
  return createSubfolder.value ? `${base}\\${subfolderName.value.trim() || preview.value?.name || ""}\\` : `${base}\\`;
});
const pickedBytes = computed(() => preview.value?.files.filter((f) => picked.value.has(f.index)).reduce((a, f) => a + f.size, 0) ?? 0);

function fmtBytes(b: number): string {
  if (b >= 1073741824) return `${(b / 1073741824).toFixed(2)} GB`;
  if (b >= 1048576) return `${(b / 1048576).toFixed(0)} MB`;
  return `${(b / 1024).toFixed(0)} KB`;
}
const pct = (r: TorrentRow) => (r.total_bytes ? (100 * r.done_bytes) / r.total_bytes : 0);
const speed = (mbps: number) => (mbps >= 1 ? `${mbps.toFixed(1)} MB/s` : `${(mbps * 1024).toFixed(0)} KB/s`);
const stateLabel: Record<TorrentRow["state"], string> = { checking: "Checking", downloading: "Downloading", seeding: "Seeding", paused: "Paused", error: "Error" };

async function refreshStatus() {
  try { status.value = await api.torrentStatus(); }
  catch (e) { toast.error(String(e)); }
}
async function refreshList() {
  if (!status.value?.running) return;
  try { rows.value = await api.torrentList(); } catch { /* engine restarting */ }
}

async function inspect(src: string) {
  if (!src.trim()) return;
  if (!status.value?.running) { toast.error(status.value?.error ?? "Choose a download folder in Settings first."); return; }
  inspecting.value = true;
  try {
    const p = await api.torrentInspect(src);
    source.value = src;
    preview.value = p;
    // Videos are ticked by default; everything else a torrent carries is left alone.
    picked.value = new Set(p.files.filter((f) => f.video).map((f) => f.index));
    if (picked.value.size === 0) picked.value = new Set(p.files.map((f) => f.index));
    // Destination defaults: the Settings folder, one subfolder named after the torrent.
    saveIn.value = status.value?.config.dir ?? "";
    createSubfolder.value = true;
    subfolderName.value = p.name;
  } catch (e) { toast.error(String(e)); }
  finally { inspecting.value = false; }
}
/** A bare info hash (40 hex / 32 base32 chars) is a magnet without the wrapper. */
function asMagnet(input: string): string | null {
  const s = input.trim();
  if (s.startsWith("magnet:")) return s;
  if (/^[0-9a-f]{40}$/i.test(s) || /^[a-z2-7]{32}$/i.test(s)) return `magnet:?xt=urn:btih:${s}`;
  return null;
}
async function addMagnet() { const m = asMagnet(magnet.value); if (!m) { toast.error("Paste a magnet link or an info hash"); return; } await inspect(m); }

// ---- stream: one step from link to player ----
const streaming = ref<string | null>(null); // what is buffering right now
const streamEnded = ref<StreamEnded | null>(null);
async function streamSource(src: string) {
  if (!src.trim()) return;
  if (!status.value?.running) { toast.error(status.value?.error ?? "Choose a download folder in Settings first."); return; }
  if (!status.value.protected) {
    const s = await api.getSettings();
    if (s.torrent_unprotected_ack !== "1") { warnUnprotected.value = true; pendingStream.value = src; return; }
  }
  streaming.value = src;
  try {
    const st = await api.torrentStream(src);
    magnet.value = "";
    const from = st.start_secs > 0 ? ` from ${hms(st.start_secs)}` : "";
    if (st.from_library) toast.success(`Already in your library — playing ${st.name}`);
    else toast.success(st.tracked ? `Streaming ${st.name}${from}` : `Opened ${st.name} with the default app`);
    await refreshList();
  } catch (e) { toast.error(String(e)); }
  finally { streaming.value = null; }
}
const pendingStream = ref<string | null>(null);
async function streamMagnet() { const m = asMagnet(magnet.value); if (!m) { toast.error("Paste a magnet link or an info hash"); return; } await streamSource(m); }
async function streamTorrentFile() {
  const p = await open({ multiple: false, filters: [{ name: "Torrent", extensions: ["torrent"] }], title: "Stream a .torrent file" });
  if (p) await streamSource(p as string);
}
async function streamRow(r: TorrentRow) {
  streaming.value = r.name;
  try {
    const st = await api.torrentStreamExisting(r.id);
    const from = st.start_secs > 0 ? ` from ${hms(st.start_secs)}` : "";
    toast.success(st.tracked ? `Streaming ${st.name}${from}` : `Opened ${st.name} with the default app`);
  } catch (e) { toast.error(String(e)); }
  finally { streaming.value = null; }
}
async function keep(id: number) {
  streamEnded.value = null;
  try { await api.torrentKeep(id); toast.success("Kept. It moves into your library when the download finishes."); await refreshList(); }
  catch (e) { toast.error(String(e)); }
}
async function discard(id: number) {
  streamEnded.value = null;
  try { await api.torrentDiscard(id); rows.value = rows.value.filter((x) => x.id !== id); toast("Stream discarded and its files deleted."); }
  catch (e) { toast.error(String(e)); }
}
async function openTorrentFile() {
  const p = await open({ multiple: false, filters: [{ name: "Torrent", extensions: ["torrent"] }], title: "Open a .torrent file" });
  if (p) await inspect(p as string);
}
async function browseSaveIn() {
  const p = await open({ directory: true, multiple: false, defaultPath: saveIn.value || undefined, title: "Save in" });
  if (p) saveIn.value = p as string;
}
function togglePick(i: number) { const s = new Set(picked.value); if (s.has(i)) s.delete(i); else s.add(i); picked.value = s; }
function pickAll(on: boolean) { picked.value = new Set(on ? preview.value?.files.map((f) => f.index) : []); }

async function startDownload() {
  if (!preview.value) return;
  if (!status.value?.protected) {
    const s = await api.getSettings();
    if (s.torrent_unprotected_ack !== "1") { warnUnprotected.value = true; return; }
  }
  await reallyStart();
}
async function acknowledgeAndStart() {
  warnUnprotected.value = false;
  await api.setSetting("torrent_unprotected_ack", "1");
  const src = pendingStream.value;
  pendingStream.value = null;
  if (src) await streamSource(src); else await reallyStart();
}
async function reallyStart() {
  if (!preview.value) return;
  starting.value = true;
  try {
    const sub = createSubfolder.value ? subfolderName.value.trim() || preview.value.name : null;
    await api.torrentAdd(source.value, [...picked.value], pickedBytes.value, saveIn.value.trim(), sub);
    toast.success(`Added ${preview.value.name}`);
    preview.value = null; magnet.value = "";
    await refreshList();
  } catch (e) { toast.error(String(e)); }
  finally { starting.value = false; }
}

async function pause(r: TorrentRow) { try { await api.torrentPause(r.id); await refreshList(); } catch (e) { toast.error(String(e)); } }
async function resume(r: TorrentRow) { try { await api.torrentResume(r.id); await refreshList(); } catch (e) { toast.error(String(e)); } }
async function remove(deleteFiles: boolean) {
  const r = removePending.value; removePending.value = null; if (!r) return;
  try { await api.torrentRemove(r.id, deleteFiles); rows.value = rows.value.filter((x) => x.id !== r.id); }
  catch (e) { toast.error(String(e)); }
}
async function play(r: TorrentRow, file: number) {
  try {
    const tracked = await api.torrentPlay(r.id, file);
    toast(tracked ? "Streaming in your player. It may take a moment to buffer." : "Opened with the default app.");
  } catch (e) { toast.error(String(e)); }
}
function toggleExpand(id: number) { const s = new Set(expanded.value); if (s.has(id)) s.delete(id); else s.add(id); expanded.value = s; }

// ---- detail panel (Files / Info / Peers / Trackers / Graphs) ----
type Tab = "files" | "info" | "peers" | "trackers" | "graphs";
const tab = ref<Record<number, Tab>>({});
const detail = ref<Record<number, TorrentDetail>>({});
const session = ref<SessionStatus | null>(null);
/** Last 90 samples (3 minutes at 2 s) of [down, up] MB/s per torrent, for the Graphs tab. */
const history = ref<Record<number, [number, number][]>>({});
const HISTORY = 90;
const tabFor = (id: number): Tab => tab.value[id] ?? "files";
function setTab(id: number, t: Tab) { tab.value = { ...tab.value, [id]: t }; }

const hms = (s: number) => {
  const h = Math.floor(s / 3600), m = Math.floor((s % 3600) / 60), sec = Math.floor(s % 60);
  return h ? `${h}h ${m}m ${sec}s` : m ? `${m}m ${sec}s` : `${sec}s`;
};
const date = (unix: number | null) => (unix ? new Date(unix * 1000).toLocaleString() : "—");
const kbps = (v: number) => (v ? `${v} KB/s` : "∞");
const count = (d: TorrentDetail, k: string) => d.peer_counts?.[k] ?? 0;

async function refreshDetails() {
  if (!status.value?.running) return;
  try { session.value = await api.torrentSessionStatus(); } catch { /* engine restarting */ }
  for (const id of expanded.value) {
    if (tabFor(id) === "files") continue;
    try { detail.value = { ...detail.value, [id]: await api.torrentDetail(id) }; }
    catch { /* removed */ }
  }
}
function sample() {
  const next: Record<number, [number, number][]> = {};
  for (const r of rows.value) {
    const prev = history.value[r.id] ?? [];
    next[r.id] = [...prev, [r.down_mbps, r.up_mbps] as [number, number]].slice(-HISTORY);
  }
  history.value = next;
}
/** SVG polyline for one series, scaled to the shared peak of both. */
function line(series: [number, number][], idx: 0 | 1, w: number, h: number): string {
  if (series.length < 2) return "";
  const peak = Math.max(0.01, ...series.map((p) => Math.max(p[0], p[1])));
  return series.map((p, i) => `${(i / (HISTORY - 1)) * w},${h - (p[idx] / peak) * (h - 2) - 1}`).join(" ");
}
const peak = (series: [number, number][]) => Math.max(0, ...series.map((p) => Math.max(p[0], p[1])));

let timer: number | undefined;
let unlistenEnded: (() => void) | undefined;
onMounted(async () => {
  await refreshStatus();
  await refreshList();
  await refreshDetails();
  // One tick at a time. Pausing or resuming a torrent holds librqbit's state
  // lock, and torrent_list needs it, so a tick can outlast the interval. Left
  // unguarded the calls stack up and all return at once, long after the data
  // they carry was current.
  let ticking = false;
  timer = window.setInterval(async () => {
    if (ticking) return;
    ticking = true;
    try {
      await refreshList();
      sample();
      await refreshDetails();
    } finally {
      ticking = false;
    }
  }, 2000);
  unlistenEnded = await api.onStreamEnded((e) => {
    if (e.ephemeral) streamEnded.value = e;
    else toast(`Stopped ${e.name} at ${hms(e.position_secs)}`);
    refreshList();
  });
});
onUnmounted(() => { clearInterval(timer); unlistenEnded?.(); });

/** A magnet: link clicked outside the app (deep link) lands here. */
async function streamFrom(url: string) {
  if (!status.value) await refreshStatus();
  magnet.value = url;
  await streamSource(url);
}
defineExpose({ reload: async () => { await refreshStatus(); await refreshList(); }, streamFrom });
</script>

<template>
  <div class="space-y-5">
    <div class="flex flex-wrap items-center gap-3">
      <h1 class="text-2xl font-semibold tracking-tight">Downloads</h1>
      <template v-if="status">
        <Badge v-if="status.running && status.protected" variant="secondary" class="gap-1 text-emerald-600 dark:text-emerald-400"><ShieldCheck class="size-3.5" /> Protected via proxy</Badge>
        <Badge v-else-if="status.running" variant="secondary" class="gap-1 text-amber-600 dark:text-amber-400"><ShieldAlert class="size-3.5" /> Unprotected — your IP is visible to peers</Badge>
        <Badge v-else variant="outline" class="gap-1 text-muted-foreground">Not running</Badge>
      </template>
      <div class="flex-1"></div>
      <Button variant="ghost" size="sm" @click="emit('go', 'settings')"><Settings /> Settings</Button>
    </div>

    <p v-if="status && !status.running" class="text-sm text-muted-foreground">
      {{ status.error ?? "Choose a download folder in Settings to enable downloads." }}
    </p>

    <div class="flex flex-wrap gap-2">
      <Input v-model="magnet" placeholder="Paste a magnet link or an info hash" class="min-w-64 flex-1" :disabled="inspecting || streaming !== null" @keydown.enter="streamMagnet" />
      <Button :disabled="inspecting || streaming !== null || !magnet.trim()" title="Play the largest video right away; keep or discard when you close the player" @click="streamMagnet"><Loader2 v-if="streaming" class="animate-spin" /><Play v-else class="fill-current" /> Stream</Button>
      <Button variant="outline" :disabled="inspecting || streaming !== null || !magnet.trim()" @click="addMagnet"><Loader2 v-if="inspecting" class="animate-spin" /><Download v-else /> Download…</Button>
      <Button variant="outline" :disabled="inspecting || streaming !== null" @click="streamTorrentFile"><FileUp /> Stream .torrent…</Button>
      <Button variant="ghost" :disabled="inspecting || streaming !== null" @click="openTorrentFile"><FileUp /> Download .torrent…</Button>
    </div>
    <p v-if="inspecting" class="text-xs text-muted-foreground">Fetching the file list from peers… magnets can take a little while.</p>
    <p v-if="streaming" class="flex items-center gap-2 text-xs text-muted-foreground"><Loader2 class="size-3.5 animate-spin" /> Finding peers and buffering the first few megabytes… the player opens by itself.</p>

    <EmptyState v-if="status?.running && rows.length === 0" title="Nothing downloading" hint="Paste a magnet link or open a .torrent file. Finished downloads move into your library folder and show up like any other file.">
      <template #icon><Download class="size-6" /></template>
    </EmptyState>

    <div v-else class="space-y-3">
      <div v-for="r in rows" :key="r.id" class="rounded-xl border bg-card p-4">
        <div class="flex items-start gap-3">
          <button class="mt-0.5 text-muted-foreground" @click="toggleExpand(r.id)">
            <ChevronDown v-if="expanded.has(r.id)" class="size-4" /><ChevronRight v-else class="size-4" />
          </button>
          <div class="min-w-0 flex-1">
            <div class="flex items-center gap-2">
              <span class="truncate font-medium">{{ r.name }}</span>
              <Badge :variant="r.state === 'error' ? 'destructive' : r.state === 'seeding' ? 'default' : 'secondary'">{{ stateLabel[r.state] }}</Badge>
              <Badge v-if="r.ephemeral" variant="outline" class="gap-1 text-violet-600 dark:text-violet-400" title="Started with Stream; not kept yet"><Play class="size-3 fill-current" /> Stream</Badge>
            </div>
            <Progress :model-value="pct(r)" class="my-2 h-1.5" />
            <div class="flex flex-wrap gap-x-4 gap-y-1 text-xs text-muted-foreground">
              <span>{{ fmtBytes(r.done_bytes) }} of {{ fmtBytes(r.total_bytes) }} ({{ pct(r).toFixed(0) }}%)</span>
              <span v-if="r.state === 'downloading'" class="flex items-center gap-1"><ArrowDown class="size-3" /> {{ speed(r.down_mbps) }}</span>
              <span v-if="r.state === 'downloading' || r.state === 'seeding'" class="flex items-center gap-1"><ArrowUp class="size-3" /> {{ speed(r.up_mbps) }}</span>
              <span v-if="r.state === 'downloading' || r.state === 'seeding'" class="flex items-center gap-1"><Users class="size-3" /> {{ r.peers }}</span>
              <span v-if="r.eta && r.state === 'downloading'">{{ r.eta }} left</span>
              <span v-if="r.error" class="text-destructive">{{ r.error }}</span>
            </div>
          </div>
          <div class="flex shrink-0 gap-1">
            <template v-if="r.ephemeral">
              <Button size="sm" variant="outline" :disabled="streaming !== null" title="Play again, resuming where you stopped" @click="streamRow(r)"><Play class="fill-current" /> Watch</Button>
              <Button size="sm" variant="ghost" title="Turn into a normal download" @click="keep(r.id)"><Save /> Keep</Button>
            </template>
            <Button v-else-if="r.state === 'paused'" size="icon-sm" variant="ghost" title="Resume" @click="resume(r)"><Play class="fill-current" /></Button>
            <Button v-else-if="!r.finished" size="icon-sm" variant="ghost" title="Pause" @click="pause(r)"><Pause class="fill-current" /></Button>
            <Button size="icon-sm" variant="ghost" class="text-muted-foreground hover:text-destructive" title="Remove" @click="removePending = r"><Trash2 /></Button>
          </div>
        </div>

        <div v-if="expanded.has(r.id)" class="mt-3 border-t pt-3">
          <Tabs :model-value="tabFor(r.id)" @update:model-value="(v) => { setTab(r.id, v as Tab); refreshDetails(); }">
            <TabsList class="h-8">
              <TabsTrigger value="files" class="text-xs"><Files class="size-3.5" /> Files</TabsTrigger>
              <TabsTrigger value="info" class="text-xs"><Info class="size-3.5" /> Info</TabsTrigger>
              <TabsTrigger value="peers" class="text-xs"><Users class="size-3.5" /> Peers</TabsTrigger>
              <TabsTrigger value="trackers" class="text-xs"><Radio class="size-3.5" /> Trackers</TabsTrigger>
              <TabsTrigger value="graphs" class="text-xs"><Activity class="size-3.5" /> Graphs</TabsTrigger>
            </TabsList>

            <TabsContent value="files" class="space-y-1">
              <div v-for="f in r.files" :key="f.index" class="flex items-center gap-3 rounded-md px-2 py-1 text-sm" :class="f.included ? '' : 'text-muted-foreground line-through'">
                <span class="min-w-0 flex-1 truncate" :title="f.path">{{ f.path }}</span>
                <span class="w-20 text-right text-xs text-muted-foreground">{{ fmtBytes(f.size) }}</span>
                <span class="w-12 text-right text-xs text-muted-foreground">{{ f.size ? (100 * f.done / f.size).toFixed(0) : 0 }}%</span>
                <Button v-if="f.video && f.included" size="xs" variant="outline" title="Stream in your player while it downloads" @click="play(r, f.index)"><Play class="fill-current" /> Play</Button>
                <span v-else class="w-[68px]"></span>
              </div>
            </TabsContent>

            <TabsContent value="info">
              <div v-if="detail[r.id]" class="text-sm">
                <div class="mb-1 text-xs font-semibold uppercase tracking-wider text-muted-foreground">Transfer</div>
                <dl class="grid grid-cols-[auto_1fr] gap-x-6 gap-y-1 sm:grid-cols-[auto_1fr_auto_1fr_auto_1fr]">
                  <dt class="text-muted-foreground">Time elapsed</dt><dd>{{ hms(detail[r.id].elapsed_secs) }}</dd>
                  <dt class="text-muted-foreground">Remaining</dt><dd>{{ fmtBytes(detail[r.id].remaining) }}<span v-if="r.eta && r.state === 'downloading'" class="text-muted-foreground"> · {{ r.eta }}</span></dd>
                  <dt class="text-muted-foreground">Wasted</dt><dd>{{ fmtBytes(detail[r.id].wasted) }}</dd>
                  <dt class="text-muted-foreground">Downloaded</dt><dd>{{ fmtBytes(detail[r.id].downloaded) }}</dd>
                  <dt class="text-muted-foreground">Uploaded</dt><dd>{{ fmtBytes(detail[r.id].uploaded) }}</dd>
                  <dt class="text-muted-foreground">Seeds</dt><dd>{{ count(detail[r.id], 'live') }} connected · {{ count(detail[r.id], 'seen') }} seen</dd>
                  <dt class="text-muted-foreground">Download speed</dt><dd>{{ speed(detail[r.id].down_mbps) }}</dd>
                  <dt class="text-muted-foreground">Upload speed</dt><dd>{{ speed(detail[r.id].up_mbps) }}</dd>
                  <dt class="text-muted-foreground">Peers</dt><dd>{{ count(detail[r.id], 'connecting') }} connecting · {{ count(detail[r.id], 'queued') }} queued</dd>
                  <dt class="text-muted-foreground">Down limit</dt><dd>{{ kbps(detail[r.id].down_limit_kbps) }}</dd>
                  <dt class="text-muted-foreground">Up limit</dt><dd>{{ kbps(detail[r.id].up_limit_kbps) }}</dd>
                  <dt class="text-muted-foreground">Share ratio</dt><dd>{{ detail[r.id].share_ratio.toFixed(3) }}</dd>
                  <dt class="text-muted-foreground">Status</dt><dd class="sm:col-span-5">{{ detail[r.id].status }}<span v-if="detail[r.id].error" class="text-destructive"> — {{ detail[r.id].error }}</span></dd>
                </dl>
                <div class="mb-1 mt-4 text-xs font-semibold uppercase tracking-wider text-muted-foreground">General</div>
                <dl class="grid grid-cols-[auto_1fr] gap-x-6 gap-y-1 sm:grid-cols-[auto_1fr_auto_1fr]">
                  <dt class="text-muted-foreground">Save as</dt><dd class="truncate sm:col-span-3" :title="detail[r.id].save_as">{{ detail[r.id].save_as }}</dd>
                  <dt class="text-muted-foreground">Total size</dt><dd>{{ fmtBytes(detail[r.id].total_size) }}</dd>
                  <dt class="text-muted-foreground">Pieces</dt><dd>{{ detail[r.id].piece_count }} × {{ fmtBytes(detail[r.id].piece_length) }}</dd>
                  <dt class="text-muted-foreground">Created on</dt><dd>{{ date(detail[r.id].created_on) }}</dd>
                  <dt class="text-muted-foreground">Created by</dt><dd>{{ detail[r.id].created_by ?? "—" }}</dd>
                  <dt class="text-muted-foreground">Hash</dt><dd class="font-mono text-xs sm:col-span-3">{{ detail[r.id].info_hash }}</dd>
                  <dt v-if="detail[r.id].comment" class="text-muted-foreground">Comment</dt><dd v-if="detail[r.id].comment" class="sm:col-span-3 break-words">{{ detail[r.id].comment }}</dd>
                </dl>
              </div>
              <p v-else class="text-xs text-muted-foreground">Loading…</p>
            </TabsContent>

            <TabsContent value="peers">
              <p v-if="!detail[r.id]" class="text-xs text-muted-foreground">Loading…</p>
              <p v-else-if="detail[r.id].peers.length === 0" class="text-xs text-muted-foreground">No connected peers right now.</p>
              <table v-else class="w-full text-xs">
                <thead class="text-muted-foreground"><tr class="text-left"><th class="py-1 font-medium">Address</th><th class="font-medium">Client</th><th class="font-medium">Via</th><th class="font-medium">State</th><th class="text-right font-medium">Downloaded</th><th class="text-right font-medium">Uploaded</th></tr></thead>
                <tbody>
                  <tr v-for="p in detail[r.id].peers" :key="p.addr" class="border-t border-border/50">
                    <td class="py-1 font-mono">{{ p.addr }}</td>
                    <td class="max-w-48 truncate" :title="p.client ?? ''">{{ p.client ?? "—" }}</td>
                    <td>{{ p.kind ?? "—" }}</td>
                    <td>{{ p.state }}</td>
                    <td class="text-right">{{ fmtBytes(p.downloaded) }}</td>
                    <td class="text-right">{{ fmtBytes(p.uploaded) }}</td>
                  </tr>
                </tbody>
              </table>
            </TabsContent>

            <TabsContent value="trackers">
              <p v-if="!detail[r.id]" class="text-xs text-muted-foreground">Loading…</p>
              <p v-else-if="detail[r.id].trackers.length === 0" class="text-xs text-muted-foreground">No trackers; peers come from DHT only.</p>
              <div v-else class="space-y-0.5 text-xs">
                <div v-for="t in detail[r.id].trackers" :key="t.url" class="flex items-center gap-3 rounded px-2 py-1" :class="t.active ? '' : 'text-muted-foreground line-through'">
                  <Badge variant="outline" class="w-12 justify-center font-mono uppercase">{{ t.protocol }}</Badge>
                  <span class="min-w-0 flex-1 truncate font-mono" :title="t.url">{{ t.url }}</span>
                  <span class="shrink-0 text-muted-foreground">{{ t.active ? "Enabled" : "Skipped behind proxy" }}</span>
                </div>
              </div>
            </TabsContent>

            <TabsContent value="graphs">
              <div class="flex items-center gap-4 text-xs text-muted-foreground">
                <span class="flex items-center gap-1"><span class="inline-block h-0.5 w-4 bg-emerald-500"></span> Download</span>
                <span class="flex items-center gap-1"><span class="inline-block h-0.5 w-4 bg-sky-500"></span> Upload</span>
                <span class="ml-auto">Last 3 min · peak {{ speed(peak(history[r.id] ?? [])) }}</span>
              </div>
              <svg viewBox="0 0 600 120" preserveAspectRatio="none" class="mt-1 h-28 w-full rounded-md border bg-muted/30">
                <line v-for="q in [30, 60, 90]" :key="q" x1="0" x2="600" :y1="q" :y2="q" class="stroke-border" stroke-width="1" />
                <polyline :points="line(history[r.id] ?? [], 0, 600, 120)" fill="none" class="stroke-emerald-500" stroke-width="2" vector-effect="non-scaling-stroke" />
                <polyline :points="line(history[r.id] ?? [], 1, 600, 120)" fill="none" class="stroke-sky-500" stroke-width="2" vector-effect="non-scaling-stroke" />
              </svg>
            </TabsContent>
          </Tabs>
        </div>
      </div>
    </div>

    <div v-if="session" class="flex flex-wrap items-center gap-x-5 gap-y-1 rounded-lg border bg-muted/40 px-3 py-1.5 text-xs text-muted-foreground">
      <span class="flex items-center gap-1.5"><Globe class="size-3.5" /> DHT: {{ session.dht ?? "—" }}</span>
      <span class="flex items-center gap-1"><ArrowDown class="size-3" /> {{ speed(session.down_mbps) }} · T: {{ fmtBytes(session.downloaded_total) }}</span>
      <span class="flex items-center gap-1"><ArrowUp class="size-3" /> {{ speed(session.up_mbps) }} · T: {{ fmtBytes(session.uploaded_total) }}</span>
      <span class="flex items-center gap-1"><Users class="size-3" /> {{ session.peers_live }} peers</span>
      <span class="ml-auto">Session {{ hms(session.uptime_secs) }}</span>
    </div>

    <Dialog :open="preview !== null" @update:open="(o) => { if (!o) preview = null; }">
      <DialogContent class="sm:max-w-2xl grid-cols-[minmax(0,1fr)]">
        <DialogHeader>
          <DialogTitle class="truncate pr-6" :title="preview?.name">{{ preview?.name }}</DialogTitle>
          <DialogDescription>Choose where it goes and what to download. Only video files can be played; nothing here is ever run.</DialogDescription>
        </DialogHeader>
        <div class="space-y-2 rounded-md border p-3">
          <div class="text-xs font-medium text-muted-foreground">Save in</div>
          <div class="flex gap-2">
            <Input v-model="saveIn" placeholder="F:\Movies" class="flex-1" />
            <Button variant="outline" @click="browseSaveIn"><FolderOpen /> Browse…</Button>
          </div>
          <label class="flex cursor-pointer items-center gap-2 text-sm">
            <input type="checkbox" class="accent-primary" v-model="createSubfolder" />
            Create subfolder
          </label>
          <div v-if="createSubfolder" class="flex items-center gap-2">
            <span class="w-12 text-xs font-medium text-muted-foreground">Name</span>
            <Input v-model="subfolderName" :placeholder="preview?.name" class="flex-1" />
          </div>
          <div class="truncate text-xs text-muted-foreground" :title="destPreview">→ {{ destPreview }}</div>
        </div>
        <div class="flex items-center gap-2 text-xs text-muted-foreground">
          <span>{{ picked.size }} of {{ preview?.files.length }} files · {{ fmtBytes(pickedBytes) }}</span>
          <div class="flex-1"></div>
          <Button size="xs" variant="ghost" @click="pickAll(true)">All</Button>
          <Button size="xs" variant="ghost" @click="pickAll(false)">None</Button>
        </div>
        <div class="max-h-80 space-y-0.5 overflow-y-auto rounded-md border p-1">
          <label v-for="f in preview?.files" :key="f.index" class="flex cursor-pointer items-center gap-3 rounded px-2 py-1.5 text-sm hover:bg-accent">
            <input type="checkbox" class="accent-primary" :checked="picked.has(f.index)" @change="togglePick(f.index)" />
            <span class="min-w-0 flex-1 truncate" :class="f.video ? '' : 'text-muted-foreground'" :title="f.path">{{ f.path }}</span>
            <span class="shrink-0 text-xs text-muted-foreground">{{ fmtBytes(f.size) }}</span>
          </label>
        </div>
        <DialogFooter>
          <Button variant="ghost" @click="preview = null">Cancel</Button>
          <Button :disabled="starting || picked.size === 0 || !saveIn.trim()" @click="startDownload"><Loader2 v-if="starting" class="animate-spin" /><Download v-else /> Start download</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>

    <AlertDialog :open="warnUnprotected" @update:open="(o) => { if (!o) warnUnprotected = false; }">
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>No proxy configured</AlertDialogTitle>
          <AlertDialogDescription>
            Without a SOCKS5 proxy, every peer in the swarm can see your IP address, the same as any ordinary torrent client. You can set one up under Settings → Downloads. Continue anyway? This is asked once.
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>Cancel</AlertDialogCancel>
          <AlertDialogAction @click="acknowledgeAndStart">Download unprotected</AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>

    <AlertDialog :open="streamEnded !== null" @update:open="(o) => { if (!o) streamEnded = null; }">
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>Keep {{ streamEnded?.name }}?</AlertDialogTitle>
          <AlertDialogDescription>
            <template v-if="streamEnded?.finished">It finished downloading while you watched. Keep moves it into your library; Discard deletes it.</template>
            <template v-else>You stopped at {{ hms(streamEnded?.position_secs ?? 0) }}. Keep carries on downloading and moves it into your library when done; Discard deletes what was fetched. Leaving it decides nothing: it stays here, paused, and Watch resumes from the same spot.</template>
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>Decide later</AlertDialogCancel>
          <Button variant="outline" @click="streamEnded && discard(streamEnded.id)"><Trash2 /> Discard</Button>
          <AlertDialogAction @click="streamEnded && keep(streamEnded.id)"><Save /> Keep in library</AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>

    <AlertDialog :open="removePending !== null" @update:open="(o) => { if (!o) removePending = null; }">
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>Remove {{ removePending?.name }}?</AlertDialogTitle>
          <AlertDialogDescription>Keep the files downloaded so far, or delete them too.</AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>Cancel</AlertDialogCancel>
          <Button variant="outline" @click="remove(false)">Keep files</Button>
          <AlertDialogAction class="bg-destructive text-white hover:bg-destructive/90" @click="remove(true)">Delete files</AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  </div>
</template>
