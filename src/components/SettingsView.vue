<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { toast } from "vue-sonner";
import { open, save } from "@tauri-apps/plugin-dialog";
import { FolderPlus, HardDrive, RefreshCw, Trash2, Check, Download, Upload, Sun, Moon, Monitor, KeyRound, Power, FileText } from "@lucide/vue";
import { api, type DetectedPlayer, type Drive, type Library, type PosterProgress, type ScanStats, type TorrentStatus } from "../lib/api";
import { applyTheme, loadTheme, type Theme } from "../lib/theme";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import { Textarea } from "@/components/ui/textarea";
import { Progress } from "@/components/ui/progress";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { AlertDialog, AlertDialogAction, AlertDialogCancel, AlertDialogContent, AlertDialogDescription, AlertDialogFooter, AlertDialogHeader, AlertDialogTitle } from "@/components/ui/alert-dialog";

const emit = defineEmits<{ scanned: [] }>();

// ---- general ----
const theme = ref<Theme>(loadTheme());
function setTheme(t: Theme) { theme.value = t; applyTheme(t); }
const flags = ref<Record<string, boolean>>({ rescan_on_startup: true, watch_folders: true, close_to_tray: true, autoplay_next: true, notify_new: true });
async function setFlag(key: string, value: boolean) { flags.value[key] = value; await api.setSetting(key, value ? "1" : "0"); }
const flagRows: [string, string, string][] = [
  ["rescan_on_startup", "Rescan libraries at startup", "Picks up files added while the app was closed."],
  ["watch_folders", "Watch folders for changes", "Updates the library when files are added, removed or renamed."],
  ["notify_new", "Windows notification for new arrivals", "A toast when the watcher finds new episodes or movies."],
  ["autoplay_next", "Play the next episode automatically", "Straight into the next one when an episode finishes."],
  ["close_to_tray", "Keep running in the system tray", "Closing the window hides it; quit from the tray."],
];

// ---- libraries ----
const libraries = ref<Library[]>([]);
const scanning = ref(false);
const lastScan = ref<ScanStats | null>(null);
const drives = ref<Drive[]>([]);
const showDrives = ref(false);
const systemDrivePending = ref<Drive | null>(null);
const ignoreDirs = ref("");
const defaultIgnored = ref<string[]>([]);
const gb = (b: number) => `${(b / 1073741824).toFixed(0)} GB`;

async function addFolder() {
  const picked = await open({ directory: true, multiple: false, title: "Choose a folder with movies or series" });
  if (!picked) return;
  await api.addLibrary(picked as string);
  await load();
  await scan();
}
async function remove(lib: Library) { await api.removeLibrary(lib.id); await load(); emit("scanned"); }
async function scan() {
  scanning.value = true;
  try { lastScan.value = await api.scanLibraries(); emit("scanned"); const s = lastScan.value; toast.success(`Scan done: ${s.files_seen} files, ${s.added} added, ${s.removed} removed`); }
  catch (e) { toast.error(String(e)); } finally { scanning.value = false; }
}
async function openDrives() { drives.value = await api.listDrives(); showDrives.value = true; }
const isSystemDrive = (d: Drive) => d.path.toUpperCase().startsWith("C:");
async function addDrive(d: Drive) { if (isSystemDrive(d)) { systemDrivePending.value = d; return; } await reallyAddDrive(d); }
async function reallyAddDrive(d: Drive) {
  systemDrivePending.value = null; showDrives.value = false;
  await api.addLibrary(d.path); await load();
  toast(`Added ${d.path}. Scanning in the background…`);
  await scan();
}
async function saveIgnore() { await api.setSetting("ignore_dirs", ignoreDirs.value.trim()); }

// ---- durations ----
const ffprobePath = ref("");
const ffprobeDetected = ref<string | null>(null);
const probing = ref(false);
const durationResult = ref("");
async function saveFfprobe() { await api.setSetting("ffprobe_path", ffprobePath.value.trim()); }
async function browseFfprobe() { const p = await open({ multiple: false, filters: [{ name: "ffprobe", extensions: ["exe"] }] }); if (p) { ffprobePath.value = p as string; await saveFfprobe(); } }
async function probeNow() { probing.value = true; durationResult.value = ""; try { await api.probeDurations(); } catch (e) { probing.value = false; toast.error(String(e)); } }

// ---- TMDB ----
const tmdbKey = ref("");
const tmdbMasked = ref<string | null>(null);
const tmdbStatus = ref("");
const tmdbBusy = ref(false);
const confirmRemove = ref(false);
const posterProgress = ref<PosterProgress | null>(null);
const fetching = ref(false);
async function connectTmdb() {
  tmdbBusy.value = true; tmdbStatus.value = "";
  try { await api.connectTmdb(tmdbKey.value); tmdbKey.value = ""; tmdbMasked.value = await api.tmdbStatus(); toast.success("TMDB connected"); }
  catch (e) { tmdbStatus.value = String(e); } finally { tmdbBusy.value = false; }
}
async function removeTmdb() { await api.disconnectTmdb(); tmdbMasked.value = null; confirmRemove.value = false; toast("TMDB key removed. Existing posters are kept."); }
async function fetchPosters(force: boolean) { try { fetching.value = true; posterProgress.value = null; await api.fetchPosters(force); } catch (e) { fetching.value = false; toast.error(String(e)); } }

// ---- player ----
const players = ref<DetectedPlayer[]>([]);
const playerKind = ref("");
const playerPath = ref("");
async function choosePlayer(p: DetectedPlayer) { playerKind.value = p.kind; playerPath.value = p.path; await savePlayer(); }
async function browsePlayer() {
  const p = await open({ multiple: false, filters: [{ name: "Executable", extensions: ["exe"] }] });
  if (!p) return;
  playerPath.value = p as string;
  const l = playerPath.value.toLowerCase();
  playerKind.value = l.includes("potplayer") ? "potplayer" : l.includes("vlc") ? "vlc" : l.includes("mpc") ? "mpc-hc" : l.includes("mpv") ? "mpv" : "custom";
  await savePlayer();
}
async function savePlayer() { await api.setSetting("player_kind", playerKind.value); await api.setSetting("player_path", playerPath.value); toast.success("Player saved"); }

// ---- downloads ----
const torrentDir = ref("");
const torrentProxy = ref("");
const torrentDown = ref("");
const torrentUp = ref("");
const torrentStatus = ref<TorrentStatus | null>(null);
const torrentBusy = ref(false);
const magnetHandler = ref(false);
async function setMagnetHandler(on: boolean) {
  magnetHandler.value = on;
  try { await api.setMagnetHandler(on); toast.success(on ? "Vortex now opens magnet links" : "Magnet links released"); }
  catch (e) { magnetHandler.value = !on; toast.error(String(e)); }
}
async function saveTorrent() {
  torrentBusy.value = true;
  try {
    await api.setSetting("torrent_dir", torrentDir.value);
    await api.setSetting("torrent_proxy", torrentProxy.value.trim());
    await api.setSetting("torrent_down_kbps", torrentDown.value.trim());
    await api.setSetting("torrent_up_kbps", torrentUp.value.trim());
    torrentStatus.value = await api.torrentRestart();
    if (torrentStatus.value.error) toast.error(torrentStatus.value.error); else toast.success("Download settings applied");
  } catch (e) { toast.error(String(e)); } finally { torrentBusy.value = false; }
}

// ---- backup / reset ----
const lastBackup = ref("");
const lastBackupPath = ref("");
const backupBusy = ref(false);
const restorePending = ref<string | null>(null);
const confirmReset = ref(false);
async function backupNow() {
  const stamp = new Date().toISOString().slice(0, 10);
  const dest = await save({ defaultPath: `vortex-backup-${stamp}.zip`, filters: [{ name: "Vortex backup", extensions: ["zip"] }] });
  if (!dest) return;
  backupBusy.value = true;
  try { const info = await api.createBackup(dest); lastBackup.value = stamp; lastBackupPath.value = info.path; toast.success(`Backup saved: ${(info.bytes / 1048576).toFixed(1)} MB, ${info.posters} images`); }
  catch (e) { toast.error(String(e)); } finally { backupBusy.value = false; }
}
async function pickRestore() { const src = await open({ multiple: false, filters: [{ name: "Vortex backup", extensions: ["zip"] }] }); if (src) restorePending.value = src as string; }
async function confirmRestore() {
  if (!restorePending.value) return;
  backupBusy.value = true;
  try { await api.restoreBackup(restorePending.value); restorePending.value = null; await load(); emit("scanned"); toast.success("Backup restored"); }
  catch (e) { toast.error(String(e)); } finally { backupBusy.value = false; }
}
async function resetWatchData() { await api.resetWatchData(); confirmReset.value = false; emit("scanned"); toast.success("All watch data cleared"); }

async function load() {
  libraries.value = await api.listLibraries();
  players.value = await api.detectPlayers();
  const s = await api.getSettings();
  playerKind.value = s.player_kind ?? ""; playerPath.value = s.player_path ?? "";
  tmdbMasked.value = await api.tmdbStatus();
  ignoreDirs.value = s.ignore_dirs ?? ""; defaultIgnored.value = await api.defaultIgnoredDirs();
  lastBackup.value = s.last_backup ?? ""; lastBackupPath.value = s.last_backup_path ?? "";
  for (const k of Object.keys(flags.value)) if (s[k] !== undefined) flags.value[k] = s[k] === "1";
  ffprobePath.value = s.ffprobe_path ?? ""; ffprobeDetected.value = await api.detectFfprobe();
  torrentDir.value = s.torrent_dir ?? ""; torrentProxy.value = s.torrent_proxy ?? "";
  torrentDown.value = s.torrent_down_kbps ?? ""; torrentUp.value = s.torrent_up_kbps ?? "";
  torrentStatus.value = await api.torrentStatus().catch(() => null);
  magnetHandler.value = s.magnet_handler === "1";
}

const unlisteners: (() => void)[] = [];
onMounted(async () => {
  await load();
  unlisteners.push(await api.onDurationsDone((p) => { probing.value = false; durationResult.value = p.total === 0 ? "All files already have a duration" : `Read ${p.found} of ${p.total} files`; }));
  unlisteners.push(await api.onPosterProgress((p) => { fetching.value = true; posterProgress.value = p; }));
  // Deliberately no emit("scanned") here. That runs onScanned, which starts
  // another poster fetch, which finishes and lands back on this handler: an
  // endless loop of fetches and toasts. App.vue already refreshes the views on
  // this same event.
  unlisteners.push(await api.onPostersDone((p) => {
    fetching.value = false;
    posterProgress.value = p;
    if (p.current) toast.error(`Poster fetch stopped: ${p.current}`);
    else if (p.total > 0) toast(`Posters: ${p.matched} of ${p.total} matched`);
  }));
});
onUnmounted(() => unlisteners.forEach((u) => u()));
</script>

<template>
  <div class="mx-auto max-w-4xl space-y-6">
    <h1 class="text-2xl font-semibold tracking-tight">Settings</h1>

    <Card>
      <CardHeader><CardTitle>Appearance &amp; behaviour</CardTitle></CardHeader>
      <CardContent class="space-y-4">
        <div class="flex items-center justify-between">
          <div class="text-sm font-medium">Theme</div>
          <div class="flex gap-1 rounded-lg bg-muted p-1">
            <Button v-for="[t, label, icon] in ([['system', 'System', Monitor], ['light', 'Light', Sun], ['dark', 'Dark', Moon]] as const)" :key="t" size="sm" :variant="theme === t ? 'default' : 'ghost'" @click="setTheme(t)"><component :is="icon" /> {{ label }}</Button>
          </div>
        </div>
        <div v-for="[key, label, hint] in flagRows" :key="key" class="flex items-center justify-between gap-4">
          <div><div class="text-sm font-medium">{{ label }}</div><div class="text-xs text-muted-foreground">{{ hint }}</div></div>
          <Switch :model-value="flags[key]" @update:model-value="(v: boolean) => setFlag(key, v)" />
        </div>
        <div class="flex items-center justify-between gap-4">
          <div><div class="text-sm font-medium">Session log</div><div class="text-xs text-muted-foreground">Everything Vortex and the torrent engine did this run, plus the previous run. Attach it to a bug report.</div></div>
          <Button variant="outline" size="sm" @click="api.revealLog()"><FileText /> Show log</Button>
        </div>
        <div class="flex justify-end"><Button variant="outline" size="sm" @click="api.quitApp()"><Power /> Quit Vortex</Button></div>
      </CardContent>
    </Card>

    <Card>
      <CardHeader><CardTitle>Libraries</CardTitle><CardDescription>Each folder is scanned recursively. External drives are remembered and shown as offline while disconnected.</CardDescription></CardHeader>
      <CardContent class="space-y-3">
        <div v-for="lib in libraries" :key="lib.id" class="flex items-center gap-3 rounded-lg border px-3 py-2">
          <span class="size-2 shrink-0 rounded-full" :class="lib.available ? 'bg-success' : 'bg-destructive'" :title="lib.available ? 'Connected' : 'Not available'"></span>
          <span class="min-w-0 flex-1 truncate text-sm" :title="lib.path">{{ lib.path }}</span>
          <Button size="sm" variant="ghost" class="text-destructive" @click="remove(lib)"><Trash2 /> Remove</Button>
        </div>
        <div class="flex flex-wrap items-center gap-2">
          <Button @click="addFolder"><FolderPlus /> Add folder</Button>
          <Button variant="outline" @click="openDrives"><HardDrive /> Add whole drive</Button>
          <Button variant="outline" :disabled="scanning || libraries.length === 0" @click="scan"><RefreshCw :class="{ 'animate-spin': scanning }" /> {{ scanning ? "Scanning…" : "Rescan all" }}</Button>
          <span class="text-xs text-muted-foreground" v-if="lastScan">{{ lastScan.files_seen }} files · {{ lastScan.added }} added · {{ lastScan.removed }} removed<template v-if="lastScan.libraries_skipped.length"> · {{ lastScan.libraries_skipped.length }} offline</template></span>
        </div>
        <div>
          <div class="mb-1 text-sm font-medium">Folders to skip</div>
          <Textarea v-model="ignoreDirs" rows="2" placeholder="One folder name per line, e.g. Incomplete" @change="saveIgnore" />
          <div class="mt-1 text-xs text-muted-foreground">Always skipped: {{ defaultIgnored.join(", ") }}, plus hidden and system folders.</div>
        </div>
      </CardContent>
    </Card>

    <Card>
      <CardHeader><CardTitle>Video player</CardTitle><CardDescription>Vortex launches this player with the resume time. PotPlayer, VLC and mpv report their exact position while playing; others are estimated from elapsed time.</CardDescription></CardHeader>
      <CardContent class="space-y-3">
        <div class="flex flex-wrap gap-2" v-if="players.length">
          <Button v-for="p in players" :key="p.kind" size="sm" :variant="playerPath === p.path ? 'default' : 'outline'" @click="choosePlayer(p)"><Check v-if="playerPath === p.path" /> {{ p.name }}</Button>
        </div>
        <div class="flex gap-2">
          <Input v-model="playerPath" placeholder="C:\Program Files\...\player.exe" @change="savePlayer" />
          <Button variant="outline" @click="browsePlayer">Browse…</Button>
          <Select v-model="playerKind" @update:model-value="savePlayer">
            <SelectTrigger class="w-44"><SelectValue placeholder="Player type" /></SelectTrigger>
            <SelectContent>
              <SelectItem value="potplayer">PotPlayer</SelectItem><SelectItem value="vlc">VLC</SelectItem><SelectItem value="mpc-hc">MPC-HC</SelectItem><SelectItem value="mpv">mpv</SelectItem><SelectItem value="custom">Other (no resume)</SelectItem>
            </SelectContent>
          </Select>
        </div>
        <p class="text-xs text-destructive" v-if="!playerPath">No player selected. Files open with the Windows default app and progress is not tracked.</p>
      </CardContent>
    </Card>

    <Card>
      <CardHeader><CardTitle>Downloads</CardTitle><CardDescription>Paste magnet links or open .torrent files on the Downloads page; Vortex never searches for content. Finished downloads move into the library folder chosen here and show up like any other file.</CardDescription></CardHeader>
      <CardContent class="space-y-3">
        <div>
          <div class="mb-1 text-sm font-medium">Default "Save in" folder</div>
          <Select v-model="torrentDir">
            <SelectTrigger class="w-full"><SelectValue placeholder="Choose a library folder" /></SelectTrigger>
            <SelectContent><SelectItem v-for="lib in libraries" :key="lib.id" :value="lib.path">{{ lib.path }}</SelectItem></SelectContent>
          </Select>
          <div class="mt-1 text-xs text-muted-foreground">{{ libraries.length ? "Each download can still pick any folder and whether to create a subfolder." : "Add a library folder first." }}</div>
        </div>
        <div>
          <div class="mb-1 text-sm font-medium">SOCKS5 proxy</div>
          <Input v-model="torrentProxy" placeholder="socks5://user:pass@host:1080 — leave empty for none" />
          <div class="mt-1 text-xs text-muted-foreground">With a proxy every connection goes through it; DHT, UDP trackers and incoming connections are switched off because they cannot. Most VPN providers offer a SOCKS5 endpoint that only answers inside the tunnel, which makes it a kill switch as well. Without one, peers see your real IP address.</div>
        </div>
        <div class="flex items-center justify-between gap-4">
          <div><div class="text-sm font-medium">Open magnet links with Vortex</div><div class="text-xs text-muted-foreground">Clicking a magnet link in your browser brings Vortex up and streams it. Takes the association from whichever client had it; switch off to give it back.</div></div>
          <Switch :model-value="magnetHandler" @update:model-value="(v: boolean) => setMagnetHandler(v)" />
        </div>
        <div class="flex gap-3">
          <div class="flex-1"><div class="mb-1 text-sm font-medium">Download limit (KB/s)</div><Input v-model="torrentDown" placeholder="0 = unlimited" /></div>
          <div class="flex-1"><div class="mb-1 text-sm font-medium">Upload limit (KB/s)</div><Input v-model="torrentUp" placeholder="0 = unlimited" /></div>
        </div>
        <div class="flex items-center gap-3">
          <Button :disabled="torrentBusy" @click="saveTorrent"><RefreshCw :class="{ 'animate-spin': torrentBusy }" /> Apply</Button>
          <span class="text-xs" v-if="torrentStatus">
            <span v-if="torrentStatus.running && torrentStatus.protected" class="text-emerald-600 dark:text-emerald-400">Running, protected via proxy.</span>
            <span v-else-if="torrentStatus.running" class="text-amber-600 dark:text-amber-400">Running unprotected: your IP is visible to peers.</span>
            <span v-else class="text-muted-foreground">{{ torrentStatus.error ?? "Not running." }}</span>
          </span>
        </div>
      </CardContent>
    </Card>

    <Card>
      <CardHeader><CardTitle>Posters &amp; details (TMDB)</CardTitle><CardDescription>Free key from themoviedb.org → Settings → API. Either the short v3 key or the long v4 read token works. Everything is cached locally.</CardDescription></CardHeader>
      <CardContent class="space-y-3">
        <div v-if="tmdbMasked" class="flex items-center gap-2">
          <Input :model-value="tmdbMasked" disabled class="max-w-xs" />
          <Badge variant="outline" class="border-success/50 text-success"><KeyRound /> Connected</Badge>
          <Button variant="ghost" size="sm" class="text-destructive" @click="confirmRemove = true">Remove</Button>
          <div class="flex-1"></div>
          <Button size="sm" :disabled="fetching" @click="fetchPosters(false)">{{ fetching ? "Fetching…" : "Fetch missing posters" }}</Button>
          <Button size="sm" variant="outline" :disabled="fetching" @click="fetchPosters(true)">Retry unmatched</Button>
        </div>
        <div v-else class="flex gap-2">
          <Input v-model="tmdbKey" type="password" placeholder="Paste your key" @keyup.enter="connectTmdb" />
          <Button :disabled="tmdbBusy || !tmdbKey.trim()" @click="connectTmdb">{{ tmdbBusy ? "Checking…" : "Add" }}</Button>
        </div>
        <p class="text-xs text-destructive" v-if="tmdbStatus">{{ tmdbStatus }}</p>
        <div v-if="posterProgress && posterProgress.total > 0">
          <Progress :model-value="100 * posterProgress.done / posterProgress.total" class="h-1.5" />
          <div class="mt-1 text-xs text-muted-foreground">{{ posterProgress.done }} / {{ posterProgress.total }} · {{ posterProgress.matched }} matched<span v-if="fetching && posterProgress.current"> · {{ posterProgress.current }}</span></div>
        </div>
      </CardContent>
    </Card>

    <Card>
      <CardHeader><CardTitle>File durations</CardTitle><CardDescription>MKV, WebM, MP4, M4V and MOV are read directly. For AVI, WMV, TS and others, point to ffprobe.exe from FFmpeg.</CardDescription></CardHeader>
      <CardContent class="space-y-3">
        <div class="flex gap-2">
          <Input v-model="ffprobePath" :placeholder="ffprobeDetected ?? 'ffprobe.exe not found — browse if you have FFmpeg'" @change="saveFfprobe" />
          <Button variant="outline" @click="browseFfprobe">Browse…</Button>
          <Button variant="outline" :disabled="probing" @click="probeNow">{{ probing ? "Reading…" : "Read missing" }}</Button>
        </div>
        <p class="text-xs text-muted-foreground" v-if="ffprobeDetected && !ffprobePath">Auto-detected: {{ ffprobeDetected }}</p>
        <p class="text-xs text-muted-foreground" v-if="durationResult">{{ durationResult }}</p>
      </CardContent>
    </Card>

    <Card>
      <CardHeader><CardTitle>Backup, restore &amp; reset</CardTitle><CardDescription>One zip with your library, history, paused positions, settings and cached posters. Video files are not included.</CardDescription></CardHeader>
      <CardContent class="flex flex-wrap items-center gap-2">
        <Button :disabled="backupBusy" @click="backupNow"><Download /> Back up now</Button>
        <Button variant="outline" :disabled="backupBusy" @click="pickRestore"><Upload /> Restore…</Button>
        <span class="text-xs text-muted-foreground" :title="lastBackupPath">{{ lastBackup ? `Last backup: ${lastBackup}` : "No backup yet" }}</span>
        <div class="flex-1"></div>
        <Button variant="outline" class="text-destructive" @click="confirmReset = true"><Trash2 /> Reset all watch data…</Button>
      </CardContent>
    </Card>

    <!-- dialogs -->
    <Dialog v-model:open="showDrives">
      <DialogContent class="max-w-xl">
        <DialogHeader><DialogTitle>Add a whole drive</DialogTitle><DialogDescription>System folders, program files and hidden folders are skipped automatically. Large drives take a while the first time.</DialogDescription></DialogHeader>
        <div class="space-y-2">
          <div v-for="d in drives" :key="d.path" class="flex items-center gap-3 rounded-lg border px-3 py-2">
            <strong class="w-10">{{ d.path.replace(/\\$/, "") }}</strong>
            <span class="w-32 truncate text-sm">{{ d.name || (d.removable ? "Removable" : "Local disk") }}</span>
            <Progress :model-value="100 * (d.total_bytes - d.free_bytes) / Math.max(1, d.total_bytes)" class="h-1.5 flex-1" />
            <span class="w-28 text-right text-xs text-muted-foreground">{{ gb(d.total_bytes - d.free_bytes) }} / {{ gb(d.total_bytes) }}</span>
            <Button size="sm" :disabled="libraries.some((l) => l.path === d.path)" @click="addDrive(d)">{{ libraries.some((l) => l.path === d.path) ? "Added" : "Add" }}</Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>

    <AlertDialog :open="!!systemDrivePending" @update:open="(v) => !v && (systemDrivePending = null)">
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>Add the Windows drive?</AlertDialogTitle>
          <AlertDialogDescription>Vortex only reads files, so this is safe. But the system drive is slow to scan and adds clutter: game clips, browser caches and half-finished downloads show up as titles. Usually it's better to add just your Videos or Downloads folder.</AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>Cancel</AlertDialogCancel>
          <Button variant="outline" @click="systemDrivePending = null; showDrives = false; addFolder()">Pick a folder instead</Button>
          <AlertDialogAction @click="reallyAddDrive(systemDrivePending!)">Add whole drive</AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>

    <AlertDialog v-model:open="confirmRemove">
      <AlertDialogContent>
        <AlertDialogHeader><AlertDialogTitle>Remove the TMDB key?</AlertDialogTitle><AlertDialogDescription>Posters already downloaded stay. New titles won't get posters or episode names until a key is added again.</AlertDialogDescription></AlertDialogHeader>
        <AlertDialogFooter><AlertDialogCancel>Cancel</AlertDialogCancel><AlertDialogAction class="bg-destructive text-white hover:bg-destructive/90" @click="removeTmdb">Remove key</AlertDialogAction></AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>

    <AlertDialog :open="!!restorePending" @update:open="(v) => !v && (restorePending = null)">
      <AlertDialogContent>
        <AlertDialogHeader><AlertDialogTitle>Restore this backup?</AlertDialogTitle><AlertDialogDescription class="break-all">{{ restorePending }}<br /><span class="mt-2 block">The current library, history and settings will be replaced. A copy of the current database is kept as vortex.db.before-restore in the app data folder.</span></AlertDialogDescription></AlertDialogHeader>
        <AlertDialogFooter><AlertDialogCancel>Cancel</AlertDialogCancel><AlertDialogAction class="bg-destructive text-white hover:bg-destructive/90" :disabled="backupBusy" @click="confirmRestore">Replace and restore</AlertDialogAction></AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>

    <AlertDialog v-model:open="confirmReset">
      <AlertDialogContent>
        <AlertDialogHeader><AlertDialogTitle>Reset all watch data?</AlertDialogTitle><AlertDialogDescription>Every title goes back to unwatched and every paused position is lost. This cannot be undone<template v-if="lastBackup">, but you have a backup from {{ lastBackup }}</template>.</AlertDialogDescription></AlertDialogHeader>
        <AlertDialogFooter><AlertDialogCancel>Cancel</AlertDialogCancel><AlertDialogAction class="bg-destructive text-white hover:bg-destructive/90" @click="resetWatchData">Reset everything</AlertDialogAction></AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  </div>
</template>
