import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface Library {
  id: number;
  path: string;
  name: string;
  available: boolean;
}

export interface MediaItem {
  id: number;
  kind: "movie" | "series";
  title: string;
  year: number | null;
  category: string | null;
  tmdb_id: number | null;
  poster_path: string | null;
  overview: string | null;
  rating: number | null;
  /** Comma-separated TMDB genre names */
  genres: string | null;
  /** Comma-separated user tags */
  tags: string | null;
  /** Comma-separated collection names */
  collections: string | null;
  episode_count: number;
  watched_count: number;
  last_watched: string | null;
  added_at: string | null;
  total_size: number;
}

export interface Episode {
  id: number;
  media_item_id: number;
  path: string;
  file_name: string;
  season: number | null;
  episode: number | null;
  size: number;
  modified: number;
  duration_secs: number | null;
  title: string | null;
  overview: string | null;
  air_date: string | null;
  still_path: string | null;
  rating: number | null;
  /** Label for bonus material, e.g. "Season 1 › Featurettes"; null for real episodes */
  extra: string | null;
  /** Sidecar subtitle paths, "|"-separated */
  subtitles: string | null;
  position_secs: number;
  completed: boolean;
  last_watched: string | null;
  available: boolean;
}

export interface ContinueItem {
  episode: Episode;
  title: string;
  kind: "movie" | "series";
  year: number | null;
  poster_path: string | null;
  tmdb_id: number | null;
}

export interface ScanStats {
  libraries_scanned: number;
  libraries_skipped: string[];
  files_seen: number;
  added: number;
  removed: number;
  renamed: number;
  extras: number;
  added_titles: string[];
}

export interface Stats {
  movies: number;
  series: number;
  episodes: number;
  total_bytes: number;
  watched_movies: number;
  watched_episodes: number;
  hours_total: number;
  hours_by_month: [string, number][];
  top_genres: [string, number][];
  storage_by_category: [string, number][];
  in_progress: [number, string, number, number, string | null][];
  stale: [number, string, number][];
}

export interface SmartList {
  id: string;
  name: string;
  kind: "movie" | "series" | "all";
  watched: "all" | "unwatched" | "watched";
  genre?: string;
  tag?: string;
  category?: string;
  minYear?: number;
  maxYear?: number;
  minRating?: number;
  /** Only titles not played for at least this many days (or never) */
  untouchedDays?: number;
}

export function subtitleCount(ep: Episode): number {
  return ep.subtitles ? ep.subtitles.split("|").filter(Boolean).length : 0;
}

export interface ScanDone {
  reason: "startup" | "watch" | "drive" | "tray" | "torrent";
  stats: ScanStats;
}

export interface DetectedPlayer {
  kind: string;
  name: string;
  path: string;
}

export interface PlaybackEnded {
  episode_id: number;
  position_secs: number;
  completed: boolean;
  /** true when the player reported its real position (VLC, mpv) */
  exact: boolean;
  next_episode_id: number | null;
  next_label: string | null;
  auto_started: boolean;
}

export interface DurationProgress {
  done: number;
  total: number;
  found: number;
}

export interface TmdbMatch {
  tmdb_id: number;
  title: string;
  year: number | null;
  overview: string | null;
  poster: string | null;
  poster_url: string | null;
  rating: number | null;
  genre_ids: number[];
}

export interface BackupInfo {
  path: string;
  bytes: number;
  posters: number;
}

export interface PosterProgress {
  done: number;
  total: number;
  matched: number;
  current: string;
}

/** Local poster file -> URL the webview can load. Cache-busted by TMDB id. */
export function posterSrc(m: { poster_path: string | null; tmdb_id: number | null }): string | null {
  return m.poster_path ? `${convertFileSrc(m.poster_path)}?v=${m.tmdb_id ?? 0}` : null;
}

export interface EpisodeHit {
  episode: Episode;
  item_title: string;
  kind: "movie" | "series";
}

export interface SearchResults {
  items: MediaItem[];
  episodes: EpisodeHit[];
}

export interface HistoryEntry {
  id: number;
  at: string;
  position_secs: number;
  completed: boolean;
  exact: boolean;
  source: "player" | "manual";
  episode: Episode;
  item_title: string;
  kind: "movie" | "series";
  media_item_id: number;
}

export interface HistoryStats {
  sessions_30d: number;
  completed_year: number;
  hours_year: number;
  total_sessions: number;
}

export interface DuplicateGroup {
  media_item_id: number;
  item_title: string;
  kind: "movie" | "series";
  year: number | null;
  season: number | null;
  episode: number | null;
  files: Episode[];
}

export interface CastMember {
  name: string;
  character: string | null;
  photo_url: string | null;
}

export interface SeasonInfo {
  season_number: number;
  name: string;
  episode_count: number;
  air_date: string | null;
  overview: string | null;
  poster_url: string | null;
}

export interface Details {
  tmdb_id: number;
  kind: "movie" | "series";
  title: string;
  original_title: string | null;
  tagline: string | null;
  overview: string | null;
  genres: string[];
  runtime_min: number | null;
  rating: number | null;
  vote_count: number | null;
  release_date: string | null;
  last_air_date: string | null;
  status: string | null;
  certification: string | null;
  language: string | null;
  backdrop_path: string | null;
  cast: CastMember[];
  directors: string[];
  writers: string[];
  creators: string[];
  companies: string[];
  trailer_youtube: string | null;
  imdb_id: string | null;
  homepage: string | null;
  tmdb_url: string;
  number_of_seasons: number | null;
  number_of_episodes: number | null;
  seasons: SeasonInfo[];
  fetched_at: string;
  collection_name: string | null;
}

export interface Tag {
  id: number;
  name: string;
  kind: "tag" | "collection";
  tmdb_collection_id: number | null;
  poster_url: string | null;
  overview: string | null;
  item_count: number;
  watched_count: number;
  first_poster: string | null;
  first_tmdb_id: number | null;
}

export interface Drive {
  path: string;
  name: string;
  total_bytes: number;
  free_bytes: number;
  removable: boolean;
}

export function collectionPoster(t: Tag): string | null {
  if (t.first_poster) return `${convertFileSrc(t.first_poster)}?v=${t.first_tmdb_id ?? 0}`;
  return t.poster_url;
}

export function backdropSrc(d: Details): string | null {
  return d.backdrop_path ? `${convertFileSrc(d.backdrop_path)}?v=${d.tmdb_id}` : null;
}

export const api = {
  stats: () => invoke<Stats>("stats"),
  listTags: (kind: "tag" | "collection") => invoke<Tag[]>("list_tags", { kind }),
  getTag: (id: number) => invoke<Tag | null>("get_tag", { id }),
  createTag: (kind: "tag" | "collection", name: string) => invoke<number>("create_tag", { kind, name }),
  renameTag: (id: number, name: string) => invoke<void>("rename_tag", { id, name }),
  deleteTag: (id: number) => invoke<void>("delete_tag", { id }),
  setItemTags: (mediaItemId: number, names: string[]) => invoke<void>("set_item_tags", { mediaItemId, names }),
  addToCollection: (tagId: number, mediaItemId: number) => invoke<void>("add_to_collection", { tagId, mediaItemId }),
  removeFromCollection: (tagId: number, mediaItemId: number) =>
    invoke<void>("remove_from_collection", { tagId, mediaItemId }),
  setCollectionOrder: (tagId: number, itemIds: number[]) => invoke<void>("set_collection_order", { tagId, itemIds }),
  collectionItems: (tagId: number) => invoke<MediaItem[]>("collection_items", { tagId }),
  listDrives: () => invoke<Drive[]>("list_drives"),
  defaultIgnoredDirs: () => invoke<string[]>("default_ignored_dirs"),
  createBackup: (path: string) => invoke<BackupInfo>("create_backup", { path }),
  restoreBackup: (path: string) => invoke<void>("restore_backup", { path }),
  onLibraryRestored: (cb: () => void): Promise<UnlistenFn> => listen("library-restored", () => cb()),
  getDetails: (mediaItemId: number, refresh = false) =>
    invoke<Details | null>("get_details", { mediaItemId, refresh }),
  listHistory: (limit = 300) => invoke<HistoryEntry[]>("list_history", { limit }),
  historyStats: () => invoke<HistoryStats>("history_stats"),
  deleteHistory: (id: number) => invoke<void>("delete_history", { id }),
  clearHistory: () => invoke<void>("clear_history"),
  findDuplicates: () => invoke<DuplicateGroup[]>("find_duplicates"),
  trashEpisode: (episodeId: number) => invoke<void>("trash_episode", { episodeId }),
  connectTmdb: (key: string) => invoke<void>("connect_tmdb", { key }),
  disconnectTmdb: () => invoke<void>("disconnect_tmdb"),
  /** Masked key tail when connected, null otherwise. */
  tmdbStatus: () => invoke<string | null>("get_tmdb_status"),
  search: (query: string) => invoke<SearchResults>("search", { query }),
  fetchEpisodeTitles: (mediaItemId: number) => invoke<number>("fetch_episode_titles", { mediaItemId }),
  fetchPosters: (force = false) => invoke<void>("fetch_posters", { force }),
  searchTmdb: (kind: "movie" | "series", query: string, year: number | null) =>
    invoke<TmdbMatch[]>("search_tmdb", { kind, query, year }),
  applyTmdbMatch: (mediaItemId: number, m: TmdbMatch) => invoke<void>("apply_tmdb_match", { mediaItemId, m }),
  testTmdbKey: (key: string) => invoke<string>("test_tmdb_key", { key }),
  onPosterProgress: (cb: (p: PosterProgress) => void): Promise<UnlistenFn> =>
    listen<PosterProgress>("posters-progress", (ev) => cb(ev.payload)),
  onPostersDone: (cb: (p: PosterProgress) => void): Promise<UnlistenFn> =>
    listen<PosterProgress>("posters-done", (ev) => cb(ev.payload)),

  listLibraries: () => invoke<Library[]>("list_libraries"),
  addLibrary: (path: string) => invoke<Library>("add_library", { path }),
  removeLibrary: (id: number) => invoke<void>("remove_library", { id }),
  scanLibraries: () => invoke<ScanStats>("scan_libraries"),

  listMedia: (kind?: "movie" | "series") => invoke<MediaItem[]>("list_media", { kind: kind ?? null }),
  listCategories: () => invoke<string[]>("list_categories"),
  setItemCategory: (mediaItemId: number, category: string | null) =>
    invoke<void>("set_item_category", { mediaItemId, category }),
  getMediaItem: (id: number) => invoke<MediaItem | null>("get_media_item", { id }),
  listEpisodes: (mediaItemId: number) => invoke<Episode[]>("list_episodes", { mediaItemId }),
  continueWatching: () => invoke<ContinueItem[]>("continue_watching"),

  setProgress: (episodeId: number, positionSecs: number, completed: boolean) =>
    invoke<void>("set_progress", { episodeId, positionSecs, completed }),
  hideFromHome: (episodeId: number) => invoke<void>("hide_from_home", { episodeId }),
  hideAllFromHome: () => invoke<number>("hide_all_from_home"),
  resetWatchData: () => invoke<void>("reset_watch_data"),
  clearProgress: (episodeId: number) => invoke<void>("clear_progress", { episodeId }),
  setItemWatched: (mediaItemId: number, watched: boolean) =>
    invoke<void>("set_item_watched", { mediaItemId, watched }),

  /** Resolves to false when no player is configured (opened with default app, untracked). */
  playEpisode: (episodeId: number) => invoke<boolean>("play_episode", { episodeId }),
  detectPlayers: () => invoke<DetectedPlayer[]>("detect_players"),
  revealPath: (path: string) => invoke<void>("reveal_path", { path }),
  probeDurations: () => invoke<void>("probe_durations"),
  detectFfprobe: () => invoke<string | null>("detect_ffprobe"),
  onScanDone: (cb: (p: ScanDone) => void): Promise<UnlistenFn> =>
    listen<ScanDone>("scan-done", (ev) => cb(ev.payload)),
  quitApp: () => invoke<void>("quit_app"),
  /** Show vortex.log in Explorer, for sending on when something breaks. */
  revealLog: () => invoke<void>("reveal_log"),
  onDurationsDone: (cb: (p: DurationProgress) => void): Promise<UnlistenFn> =>
    listen<DurationProgress>("durations-done", (ev) => cb(ev.payload)),

  getSettings: () => invoke<Record<string, string>>("get_settings"),
  setSetting: (key: string, value: string) => invoke<void>("set_setting", { key, value }),

  onPlaybackEnded: (cb: (e: PlaybackEnded) => void): Promise<UnlistenFn> =>
    listen<PlaybackEnded>("playback-ended", (ev) => cb(ev.payload)),

  // ---- torrents ----
  torrentStatus: () => invoke<TorrentStatus>("torrent_status"),
  torrentRestart: () => invoke<TorrentStatus>("torrent_restart"),
  /** Resolves the file list; for magnets this waits for metadata from peers. */
  torrentInspect: (source: string) => invoke<TorrentPreview>("torrent_inspect", { source }),
  torrentAdd: (source: string, files: number[], neededBytes: number, saveIn: string, subfolder: string | null) =>
    invoke<number>("torrent_add", { source, files, neededBytes, saveIn, subfolder }),
  torrentList: () => invoke<TorrentRow[]>("torrent_list"),
  torrentPause: (id: number) => invoke<void>("torrent_pause", { id }),
  torrentResume: (id: number) => invoke<void>("torrent_resume", { id }),
  torrentRemove: (id: number, deleteFiles: boolean) => invoke<void>("torrent_remove", { id, deleteFiles }),
  /** Streams the file in the configured player. False when opened with the default app. */
  torrentPlay: (id: number, file: number) => invoke<boolean>("torrent_play", { id, file }),
  onTorrentDone: (cb: (p: TorrentDone) => void): Promise<UnlistenFn> =>
    listen<TorrentDone>("torrent-done", (ev) => cb(ev.payload)),
  torrentDetail: (id: number) => invoke<TorrentDetail>("torrent_detail", { id }),
  torrentSessionStatus: () => invoke<SessionStatus | null>("torrent_session_status"),
  /** One-step stream: resolve, pick the largest video, buffer, play. Resolves once the player opens. */
  torrentStream: (source: string) => invoke<StreamStarted>("torrent_stream", { source }),
  torrentStreamExisting: (id: number) => invoke<StreamStarted>("torrent_stream_existing", { id }),
  torrentKeep: (id: number) => invoke<void>("torrent_keep", { id }),
  torrentDiscard: (id: number) => invoke<void>("torrent_discard", { id }),
  onStreamEnded: (cb: (p: StreamEnded) => void): Promise<UnlistenFn> =>
    listen<StreamEnded>("stream-ended", (ev) => cb(ev.payload)),
  /** Register or release the magnet: link association for Vortex. */
  setMagnetHandler: (on: boolean) => invoke<void>("set_magnet_handler", { on }),
  /** Magnet links that opened the app before the Downloads page was listening. */
  pendingOpenUrls: () => invoke<string[]>("pending_open_urls"),
  /** A magnet: or vortex:// link was clicked outside the app. */
  onOpenUrl: (cb: (magnet: string) => void): Promise<UnlistenFn> =>
    listen<string>("open-url", (ev) => cb(ev.payload)),
};

// ---- torrents ----

export interface TorrentConfig {
  /** socks5://host:port, empty when none */
  proxy: string;
  /** Library folder finished downloads move into */
  dir: string;
  down_kbps: number;
  up_kbps: number;
}

export interface TorrentStatus {
  running: boolean;
  /** True when every connection goes through the proxy */
  protected: boolean;
  config: TorrentConfig;
  error: string | null;
}

export interface PreviewFile {
  index: number;
  path: string;
  size: number;
  video: boolean;
}

export interface TorrentPreview {
  name: string;
  info_hash: string;
  total_bytes: number;
  files: PreviewFile[];
}

export interface TorrentFile {
  index: number;
  path: string;
  size: number;
  done: number;
  included: boolean;
  video: boolean;
}

export interface TorrentRow {
  id: number;
  name: string;
  info_hash: string;
  state: "checking" | "downloading" | "seeding" | "paused" | "error";
  error: string | null;
  done_bytes: number;
  total_bytes: number;
  uploaded_bytes: number;
  /** Megabytes per second */
  down_mbps: number;
  up_mbps: number;
  peers: number;
  eta: string | null;
  finished: boolean;
  /** Started with Stream and not yet kept or discarded */
  ephemeral: boolean;
  files: TorrentFile[];
}

export interface TorrentDone {
  name: string;
  /** Names of the entries moved into the library folder */
  moved: string[];
}

export interface PeerRow {
  addr: string;
  client: string | null;
  state: string;
  kind: string | null;
  downloaded: number;
  uploaded: number;
}

export interface TrackerRow {
  url: string;
  protocol: "http" | "https" | "udp" | "other";
  /** False when the engine will not contact it (UDP behind a proxy) */
  active: boolean;
}

export interface TorrentDetail {
  id: number;
  elapsed_secs: number;
  downloaded: number;
  remaining: number;
  wasted: number;
  uploaded: number;
  down_mbps: number;
  up_mbps: number;
  down_limit_kbps: number;
  up_limit_kbps: number;
  share_ratio: number;
  status: string;
  error: string | null;
  /** Peer counts by state, e.g. { live, seen, connecting, queued, dead } */
  peer_counts: Record<string, number>;
  save_as: string;
  total_size: number;
  piece_count: number;
  piece_length: number;
  /** Unix seconds */
  created_on: number | null;
  created_by: string | null;
  comment: string | null;
  info_hash: string;
  peers: PeerRow[];
  trackers: TrackerRow[];
}

export interface SessionStatus {
  dht: string | null;
  down_mbps: number;
  up_mbps: number;
  downloaded_total: number;
  uploaded_total: number;
  peers_live: number;
  uptime_secs: number;
  protected: boolean;
}

export interface StreamStarted {
  id: number;
  name: string;
  file: number;
  /** Resumed from this many seconds in */
  start_secs: number;
  /** False when no player is configured and the URL went to the default app */
  tracked: boolean;
  /** The torrent had already finished and moved: this played the library file */
  from_library: boolean;
}

export interface StreamEnded {
  id: number;
  name: string;
  position_secs: number;
  duration_secs: number | null;
  finished: boolean;
  /** Ask Keep / Discard only for streams */
  ephemeral: boolean;
  /** True when the position came from the player rather than a clock */
  exact: boolean;
}
