<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { toast } from "vue-sonner";
import { openUrl } from "@tauri-apps/plugin-opener";
import { KeyRound, ExternalLink, FilePen, Square, SearchX } from "@lucide/vue";
import { api, type MediaItem, type PosterProgress, type TmdbStore } from "../lib/api";
import RenameDialog from "./RenameDialog.vue";
import EmptyState from "./EmptyState.vue";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import { Badge } from "@/components/ui/badge";
import { Progress } from "@/components/ui/progress";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { AlertDialog, AlertDialogAction, AlertDialogCancel, AlertDialogContent, AlertDialogDescription, AlertDialogFooter, AlertDialogHeader, AlertDialogTitle } from "@/components/ui/alert-dialog";

/**
 * Everything TMDb in one place: the key, how titles are matched, what is
 * kept locally, the fetch tools, the titles still unmatched, and renaming
 * files to TMDb names.
 */
const emit = defineEmits<{ open: [id: number]; changed: [] }>();

// ---- connection ----
const tmdbKey = ref("");
const tmdbMasked = ref<string | null>(null);
const tmdbStatus = ref("");
const tmdbBusy = ref(false);
const confirmRemove = ref(false);
async function connect() {
  tmdbBusy.value = true; tmdbStatus.value = "";
  try { await api.connectTmdb(tmdbKey.value); tmdbKey.value = ""; tmdbMasked.value = await api.tmdbStatus(); toast.success("TMDb connected"); emit("changed"); }
  catch (e) { tmdbStatus.value = String(e); } finally { tmdbBusy.value = false; }
}
async function removeKey() { await api.disconnectTmdb(); tmdbMasked.value = null; confirmRemove.value = false; toast("TMDb key removed. Existing posters are kept."); }

// ---- matching rules ----
// Defaults mirror the backend: auto-match and the year fallback are on,
// adult titles are off, English, details kept 90 days.
const flags = ref<Record<string, boolean>>({ tmdb_auto_match: true, tmdb_adult: false, tmdb_year_fallback: true });
const flagRows: [string, string, string][] = [
  ["tmdb_auto_match", "Match new titles automatically", "After every scan, titles without a poster are looked up on TMDb. Off: only Fix match and the buttons below ask TMDb."],
  ["tmdb_adult", "Include adult titles", "TMDb hides films it flags as adult unless asked, so they never matched (Monella, for one). Fix match always shows them; this lets automatic matching pick them too."],
  ["tmdb_year_fallback", "Retry without the year", "When nothing matches the year in the file name, search the title alone. Off: a wrong year in a file name leaves the title unmatched for you to fix by hand."],
];
async function setFlag(key: string, value: boolean) { flags.value[key] = value; await api.setSetting(key, value ? "1" : "0"); }

const LANGUAGES: [string, string][] = [
  ["en-US", "English (US)"], ["en-GB", "English (UK)"], ["de-DE", "Deutsch"], ["fr-FR", "Français"], ["es-ES", "Español"],
  ["it-IT", "Italiano"], ["pt-BR", "Português (Brasil)"], ["ru-RU", "Русский"], ["ja-JP", "日本語"], ["ko-KR", "한국어"],
  ["zh-CN", "中文 (简体)"], ["hi-IN", "हिन्दी"], ["ta-IN", "தமிழ்"], ["te-IN", "తెలుగు"], ["ml-IN", "മലയാളം"], ["kn-IN", "ಕನ್ನಡ"],
];
const language = ref("en-US");
async function setLanguage(v: string) {
  language.value = v;
  await api.setSetting("tmdb_language", v);
  toast("Language saved. Titles already fetched keep their text until you press Refresh on their page.");
}
const cacheDays = ref("90");
async function saveCacheDays() {
  const n = Math.max(0, Math.floor(Number(cacheDays.value) || 0));
  cacheDays.value = String(n);
  await api.setSetting("tmdb_cache_days", String(n));
}

// ---- API options ----
// Everything TMDb's API accepts on the calls Vortex makes, with the app's
// defaults. Image sizes are the ones TMDb's configuration lists.
const POSTER_SIZES = ["w92", "w154", "w185", "w342", "w500", "w780", "original"];
const BACKDROP_SIZES = ["w300", "w780", "w1280", "original"];
const STILL_SIZES = ["w92", "w185", "w300", "original"];
const PROFILE_SIZES = ["w45", "w185", "h632", "original"];
const apiText = ref<Record<string, string>>({
  tmdb_region: "", tmdb_year_mode: "any", tmdb_cert_country: "US",
  tmdb_poster_size: "w342", tmdb_backdrop_size: "w1280", tmdb_still_size: "w300", tmdb_profile_size: "w185",
  tmdb_delay_ms: "120", tmdb_timeout_secs: "20",
});
// The Select component refuses an empty value, so "no region" travels as
// this sentinel and is stored as an empty setting.
const NO_REGION = "default";
async function setApi(key: string, value: string) {
  const stored = key === "tmdb_region" && value === NO_REGION ? "" : value.trim();
  apiText.value[key] = stored;
  await api.setSetting(key, stored);
}
const COUNTRIES: [string, string][] = [
  [NO_REGION, "TMDb default"], ["US", "United States"], ["GB", "United Kingdom"], ["IN", "India"], ["DE", "Germany"], ["FR", "France"], ["ES", "Spain"],
  ["IT", "Italy"], ["BR", "Brazil"], ["RU", "Russia"], ["JP", "Japan"], ["KR", "South Korea"], ["CN", "China"], ["AU", "Australia"], ["CA", "Canada"],
];

// ---- fetch tools ----
const posterProgress = ref<PosterProgress | null>(null);
const fetching = ref(false);
const store = ref<TmdbStore | null>(null);
async function fetchPosters(force: boolean) { try { fetching.value = true; posterProgress.value = null; await api.fetchPosters(force); } catch (e) { fetching.value = false; toast.error(String(e)); } }
async function refreshStore() { store.value = await api.tmdbStore().catch(() => null); }

// ---- unmatched titles ----
const unmatched = ref<MediaItem[]>([]);
const SHOWN = 150;
const shown = computed(() => unmatched.value.slice(0, SHOWN));
async function loadUnmatched() {
  unmatched.value = (await api.listMedia()).filter((m) => !m.tmdb_id);
}

// ---- rename to TMDb names ----
const renameOpen = ref(false);
const canUndoRename = ref(false);
async function refreshUndo() { canUndoRename.value = await api.renameCanUndo().catch(() => false); }
async function afterRename() { await refreshUndo(); emit("changed"); }
async function undoRename() {
  try { const n = await api.renameUndo(); toast(`Rename undone (${n} change${n === 1 ? "" : "s"} reversed)`); }
  catch (e) { toast.error(String(e)); }
  await afterRename();
}

async function load() {
  const s = await api.getSettings();
  tmdbMasked.value = await api.tmdbStatus();
  for (const k of Object.keys(flags.value)) if (s[k] !== undefined) flags.value[k] = s[k] === "1";
  language.value = s.tmdb_language || "en-US";
  cacheDays.value = s.tmdb_cache_days ?? "90";
  for (const k of Object.keys(apiText.value)) if (s[k] !== undefined && (s[k] !== "" || k === "tmdb_region")) apiText.value[k] = s[k];
  await Promise.all([refreshStore(), refreshUndo(), loadUnmatched()]);
}

const unlisteners: (() => void)[] = [];
let unmounted = false;
onMounted(async () => {
  const keep = (u: () => void) => { if (unmounted) u(); else unlisteners.push(u); };
  keep(await api.onPosterProgress((p) => { fetching.value = true; posterProgress.value = p; }));
  keep(await api.onPostersDone((p) => {
    fetching.value = false;
    posterProgress.value = p;
    if (p.current) toast.error(`Poster fetch stopped: ${p.current}`);
    else if (p.total > 0) toast(`Posters: ${p.matched} of ${p.total} matched`);
    refreshStore();
    loadUnmatched();
  }));
  if (!unmounted) await load();
});
onUnmounted(() => { unmounted = true; unlisteners.forEach((u) => u()); });
defineExpose({ reload: load });
</script>

<template>
  <div class="mx-auto max-w-4xl space-y-6">
    <div>
      <h1 class="text-[2.6rem] font-black leading-none tracking-[-0.04em]">TMDb</h1>
      <p class="mt-2 text-sm text-muted-foreground">Posters, backdrops, ratings, cast, episode names and collections come from The Movie Database. Everything fetched is kept on this PC. This product uses the TMDB API but is not endorsed or certified by TMDB.</p>
    </div>

    <Card>
      <CardHeader><CardTitle>Connection</CardTitle><CardDescription>A free key from your TMDb account. Either the short v3 key or the long v4 read token works. The key is stored in the database and never shown again.</CardDescription></CardHeader>
      <CardContent class="space-y-3">
        <div v-if="tmdbMasked" class="flex flex-wrap items-center gap-2">
          <Input :model-value="tmdbMasked" disabled class="max-w-xs" />
          <Badge variant="outline" class="border-success/50 text-success"><KeyRound /> Connected</Badge>
          <Button variant="ghost" size="sm" class="text-destructive" @click="confirmRemove = true">Remove key</Button>
        </div>
        <div v-else class="flex gap-2">
          <Input v-model="tmdbKey" type="password" placeholder="Paste your key" @keyup.enter="connect" />
          <Button :disabled="tmdbBusy || !tmdbKey.trim()" @click="connect">{{ tmdbBusy ? "Checking…" : "Connect" }}</Button>
        </div>
        <p class="text-xs text-destructive" v-if="tmdbStatus">{{ tmdbStatus }}</p>
        <Button variant="link" size="sm" class="h-auto p-0 text-xs" @click="openUrl('https://www.themoviedb.org/settings/api')"><ExternalLink /> Get a key at themoviedb.org → Settings → API</Button>
      </CardContent>
    </Card>

    <Card>
      <CardHeader><CardTitle>Matching</CardTitle><CardDescription>How file names are looked up. Titles are searched by the name and year read from the file; the first result is taken. Fix match on a title's page always overrides.</CardDescription></CardHeader>
      <CardContent class="space-y-4">
        <div v-for="[key, label, hint] in flagRows" :key="key" class="flex items-center justify-between gap-4">
          <div><div class="text-sm font-medium">{{ label }}</div><div class="text-xs text-muted-foreground">{{ hint }}</div></div>
          <Switch :model-value="flags[key]" @update:model-value="(v: boolean) => setFlag(key, v)" />
        </div>
        <div class="flex items-center justify-between gap-4">
          <div><div class="text-sm font-medium">Language</div><div class="text-xs text-muted-foreground">For titles, overviews, genres and episode names. Searching finds a title in any language regardless.</div></div>
          <Select :model-value="language" @update:model-value="(v) => v && setLanguage(String(v))">
            <SelectTrigger class="w-48"><SelectValue /></SelectTrigger>
            <SelectContent><SelectItem v-for="[code, name] in LANGUAGES" :key="code" :value="code">{{ name }}</SelectItem></SelectContent>
          </Select>
        </div>
        <div class="flex items-center justify-between gap-4">
          <div><div class="text-sm font-medium">Refresh details after</div><div class="text-xs text-muted-foreground">Days before a title's cast, rating and season list are fetched again on their own. 0 keeps them for ever; Refresh on a title's page always fetches.</div></div>
          <div class="flex items-center gap-2"><Input v-model="cacheDays" type="number" min="0" class="w-24" @change="saveCacheDays" /><span class="text-sm text-muted-foreground">days</span></div>
        </div>
      </CardContent>
    </Card>

    <Card>
      <CardHeader><CardTitle>API options</CardTitle><CardDescription>Every parameter TMDb's API accepts on the calls Vortex makes. The defaults are what Vortex used until now; change what you like.</CardDescription></CardHeader>
      <CardContent class="space-y-4">
        <div class="flex items-center justify-between gap-4">
          <div><div class="text-sm font-medium">Search region <span class="font-mono text-xs text-muted-foreground">region</span></div><div class="text-xs text-muted-foreground">Movie searches: which country's release dates the year filter uses. A film released in India a year after the US matches its Indian year with IN.</div></div>
          <Select :model-value="apiText.tmdb_region || NO_REGION" @update:model-value="(v) => v && setApi('tmdb_region', String(v))">
            <SelectTrigger class="w-48"><SelectValue /></SelectTrigger>
            <SelectContent><SelectItem v-for="[code, name] in COUNTRIES" :key="code" :value="code">{{ name }}</SelectItem></SelectContent>
          </Select>
        </div>
        <div class="flex items-center justify-between gap-4">
          <div><div class="text-sm font-medium">Year filter <span class="font-mono text-xs text-muted-foreground">year · primary_release_year</span></div><div class="text-xs text-muted-foreground">Any release: a film counts for every year it was released somewhere. Original release: only the year of its first release.</div></div>
          <Select :model-value="apiText.tmdb_year_mode" @update:model-value="(v) => v && setApi('tmdb_year_mode', String(v))">
            <SelectTrigger class="w-48"><SelectValue /></SelectTrigger>
            <SelectContent><SelectItem value="any">Any release (year)</SelectItem><SelectItem value="primary">Original release</SelectItem></SelectContent>
          </Select>
        </div>
        <div class="flex items-center justify-between gap-4">
          <div><div class="text-sm font-medium">Certification country <span class="font-mono text-xs text-muted-foreground">release_dates · content_ratings</span></div><div class="text-xs text-muted-foreground">Whose age rating is shown on title pages (PG-13, 15, U/A). Another country's is used when that one has none.</div></div>
          <Select :model-value="apiText.tmdb_cert_country" @update:model-value="(v) => v && setApi('tmdb_cert_country', String(v))">
            <SelectTrigger class="w-48"><SelectValue /></SelectTrigger>
            <SelectContent><SelectItem v-for="[code, name] in COUNTRIES.filter((c) => c[0] !== NO_REGION)" :key="code" :value="code">{{ name }}</SelectItem></SelectContent>
          </Select>
        </div>
        <div class="grid gap-3 sm:grid-cols-2">
          <div v-for="[key, label, sizes, hint] in ([
            ['tmdb_poster_size', 'Poster size', POSTER_SIZES, 'Downloaded and kept on this PC; larger is sharper on the hero and costs disk space. Applies to posters fetched from now on.'],
            ['tmdb_backdrop_size', 'Backdrop size', BACKDROP_SIZES, 'The wide picture behind title pages and the Home billboard.'],
            ['tmdb_still_size', 'Episode still size', STILL_SIZES, 'Episode thumbnails; loaded from TMDb when shown.'],
            ['tmdb_profile_size', 'Cast photo size', PROFILE_SIZES, 'Round portraits in the cast row.'],
          ] as [string, string, string[], string][])" :key="key" class="flex items-center justify-between gap-3 rounded-lg border p-3">
            <div><div class="text-sm font-medium">{{ label }}</div><div class="text-xs text-muted-foreground">{{ hint }}</div></div>
            <Select :model-value="apiText[key]" @update:model-value="(v) => v && setApi(key, String(v))">
              <SelectTrigger class="w-28"><SelectValue /></SelectTrigger>
              <SelectContent><SelectItem v-for="sz in sizes" :key="sz" :value="sz">{{ sz }}</SelectItem></SelectContent>
            </Select>
          </div>
        </div>
        <div class="grid gap-3 sm:grid-cols-2">
          <div class="flex items-center justify-between gap-3 rounded-lg border p-3">
            <div><div class="text-sm font-medium">Pause between calls</div><div class="text-xs text-muted-foreground">During a long fetch. TMDb allows about 50 calls a second; a larger pause is gentler on a slow connection.</div></div>
            <div class="flex items-center gap-1"><Input :model-value="apiText.tmdb_delay_ms" type="number" min="0" max="10000" class="w-24" @change="(e: Event) => setApi('tmdb_delay_ms', (e.target as HTMLInputElement).value)" /><span class="text-xs text-muted-foreground">ms</span></div>
          </div>
          <div class="flex items-center justify-between gap-3 rounded-lg border p-3">
            <div><div class="text-sm font-medium">Call timeout</div><div class="text-xs text-muted-foreground">How long one request may take before it counts as failed. 5 to 120 seconds.</div></div>
            <div class="flex items-center gap-1"><Input :model-value="apiText.tmdb_timeout_secs" type="number" min="5" max="120" class="w-24" @change="(e: Event) => setApi('tmdb_timeout_secs', (e.target as HTMLInputElement).value)" /><span class="text-xs text-muted-foreground">s</span></div>
          </div>
        </div>
        <p class="text-xs text-muted-foreground">Vortex also always sends <span class="font-mono">append_to_response=credits,videos,external_ids,release_dates</span> (movies) or <span class="font-mono">content_ratings</span> (series) with details, and never asks for keywords, reviews, recommendations or watch providers, since nothing in the app shows them.</p>
      </CardContent>
    </Card>

    <Card>
      <CardHeader><CardTitle>Posters &amp; details</CardTitle><CardDescription>Fetch what is missing now, or try again for titles TMDb had nothing for last time.</CardDescription></CardHeader>
      <CardContent class="space-y-3">
        <div class="flex flex-wrap items-center gap-2">
          <Button size="sm" :disabled="fetching || !tmdbMasked" @click="fetchPosters(false)">{{ fetching ? "Fetching…" : "Fetch missing posters" }}</Button>
          <Button v-if="fetching" size="sm" variant="outline" @click="api.cancelPosters()"><Square class="fill-current" /> Stop</Button>
          <Button v-else size="sm" variant="outline" :disabled="!tmdbMasked" @click="fetchPosters(true)">Retry unmatched</Button>
          <span class="text-xs text-muted-foreground" v-if="!tmdbMasked">Connect a key first.</span>
        </div>
        <div v-if="posterProgress && posterProgress.total > 0">
          <Progress :model-value="100 * posterProgress.done / posterProgress.total" class="h-1.5" />
          <div class="mt-1 text-xs text-muted-foreground">{{ posterProgress.done }} / {{ posterProgress.total }} · {{ Math.round(100 * posterProgress.done / posterProgress.total) }}% · {{ posterProgress.matched }} matched<span v-if="fetching && posterProgress.current"> · {{ posterProgress.current }}</span></div>
        </div>
        <p v-if="store" class="text-xs text-muted-foreground">
          Saved on this PC: {{ store.titles }} matched title{{ store.titles === 1 ? "" : "s" }}, {{ store.details }} detail pages, {{ store.images }} images ({{ (store.image_bytes / 1048576).toFixed(0) }} MB).
          Folders you switch back to get their posters and details from here, without asking TMDb.
        </p>
      </CardContent>
    </Card>

    <Card>
      <CardHeader><CardTitle>Unmatched titles <span class="text-base font-normal text-muted-foreground">{{ unmatched.length }}</span></CardTitle><CardDescription>Titles with no TMDb match yet. Open one and use Fix match to search by hand; a wrong file name is the usual reason.</CardDescription></CardHeader>
      <CardContent>
        <EmptyState v-if="unmatched.length === 0" title="Everything is matched" hint="Every title in the library has a TMDb entry."><template #icon><SearchX class="size-6" /></template></EmptyState>
        <div v-else class="grid gap-1 sm:grid-cols-2">
          <button v-for="m in shown" :key="m.id" class="flex items-center gap-2 rounded-lg px-2 py-1.5 text-left text-sm hover:bg-accent" @click="emit('open', m.id)">
            <span class="min-w-0 flex-1 truncate font-medium">{{ m.title }}</span>
            <span class="shrink-0 text-xs text-muted-foreground">{{ [m.year, m.kind === "movie" ? "Movie" : "Series", m.category].filter(Boolean).join(" · ") }}</span>
          </button>
        </div>
        <p v-if="unmatched.length > SHOWN" class="mt-2 text-xs text-muted-foreground">And {{ unmatched.length - SHOWN }} more.</p>
      </CardContent>
    </Card>

    <Card>
      <CardHeader><CardTitle>File names</CardTitle><CardDescription>Rename files and folders to their TMDb names: Inception (2010).mkv, Dark (2017) - S01E01 - Secrets.mkv, anime included. Every change is shown before anything happens, and the last batch can be undone.</CardDescription></CardHeader>
      <CardContent class="flex flex-wrap items-center gap-2">
        <Button size="sm" variant="outline" :disabled="!tmdbMasked" @click="renameOpen = true"><FilePen /> Review renames…</Button>
        <Button size="sm" variant="ghost" :disabled="!canUndoRename" @click="undoRename">Undo last rename</Button>
      </CardContent>
    </Card>
    <RenameDialog v-model:open="renameOpen" :ids="null" @done="afterRename" />

    <AlertDialog v-model:open="confirmRemove">
      <AlertDialogContent>
        <AlertDialogHeader><AlertDialogTitle>Remove the TMDb key?</AlertDialogTitle><AlertDialogDescription>Posters already downloaded stay. New titles won't get posters or episode names until a key is added again.</AlertDialogDescription></AlertDialogHeader>
        <AlertDialogFooter><AlertDialogCancel>Cancel</AlertDialogCancel><AlertDialogAction class="bg-destructive text-white hover:bg-destructive/90" @click="removeKey">Remove key</AlertDialogAction></AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  </div>
</template>
