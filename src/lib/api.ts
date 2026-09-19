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
  reason: "startup" | "watch" | "drive" | "tray";
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
  onDurationsDone: (cb: (p: DurationProgress) => void): Promise<UnlistenFn> =>
    listen<DurationProgress>("durations-done", (ev) => cb(ev.payload)),

  getSettings: () => invoke<Record<string, string>>("get_settings"),
  setSetting: (key: string, value: string) => invoke<void>("set_setting", { key, value }),

  onPlaybackEnded: (cb: (e: PlaybackEnded) => void): Promise<UnlistenFn> =>
    listen<PlaybackEnded>("playback-ended", (ev) => cb(ev.payload)),
};
