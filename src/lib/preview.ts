/**
 * Sample library for designing in a plain browser (`npm run dev`), where the
 * Tauri backend is absent. Only ever loaded when `import.meta.env.DEV` and no
 * Tauri bridge exists; the shipped app never reaches this file.
 */
import type { ContinueItem, Details, Episode, MediaItem, Tag } from "./api";

export const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
export const previewMode = import.meta.env.DEV && !inTauri;

/** Poster or backdrop art: a gradient with the title set in it. */
function art(title: string, hue: number, wide = false): string {
  const [w, h] = wide ? [1280, 720] : [400, 600];
  const words = title.split(" ");
  const lines: string[] = [];
  for (const word of words) {
    const last = lines[lines.length - 1];
    if (last && (last + " " + word).length <= (wide ? 22 : 12)) lines[lines.length - 1] = last + " " + word;
    else lines.push(word);
  }
  const size = wide ? 84 : 46;
  const text = lines
    .map((l, i) => `<text x="${wide ? 90 : 30}" y="${(wide ? 470 : 470) + (i - lines.length + 1) * size * 1.05}" font-family="Inter, Segoe UI, sans-serif" font-weight="800" font-size="${size}" fill="white" fill-opacity="0.92" letter-spacing="-1">${l.replace(/&/g, "&amp;")}</text>`)
    .join("");
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${w}" height="${h}" viewBox="0 0 ${w} ${h}">
<defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1">
<stop offset="0" stop-color="hsl(${hue} 70% 46%)"/><stop offset="1" stop-color="hsl(${(hue + 50) % 360} 65% 14%)"/></linearGradient>
<radialGradient id="r" cx="0.75" cy="0.25" r="0.6"><stop offset="0" stop-color="hsl(${(hue + 30) % 360} 90% 70%)" stop-opacity="0.55"/><stop offset="1" stop-color="black" stop-opacity="0"/></radialGradient></defs>
<rect width="${w}" height="${h}" fill="url(#g)"/><rect width="${w}" height="${h}" fill="url(#r)"/>
<circle cx="${w * 0.72}" cy="${h * 0.3}" r="${h * 0.22}" fill="white" fill-opacity="0.07"/>
<rect y="${h * 0.55}" width="${w}" height="${h * 0.45}" fill="black" fill-opacity="0.25"/>${text}</svg>`;
  return `data:image/svg+xml;utf8,${encodeURIComponent(svg)}`;
}

const T = [
  ["Dune: Part Two", "movie", 2024, 8.2, "Science Fiction, Adventure", 28],
  ["Oppenheimer", "movie", 2023, 8.1, "Drama, History", 18],
  ["The Boy and the Heron", "movie", 2023, 7.4, "Animation, Fantasy", 200],
  ["Past Lives", "movie", 2023, 7.8, "Drama, Romance", 330],
  ["Dark", "series", 2017, 8.4, "Mystery, Sci-Fi & Fantasy", 150],
  ["Shōgun", "series", 2024, 8.6, "Drama, War & Politics", 8],
  ["Attack on Titan", "series", 2013, 8.7, "Animation, Action & Adventure", 100],
  ["Blade Runner 2049", "movie", 2017, 7.6, "Science Fiction, Drama", 22],
  ["Spirited Away", "movie", 2001, 8.5, "Animation, Family, Fantasy", 175],
  ["The Bear", "series", 2022, 8.2, "Drama, Comedy", 12],
  ["Arrival", "movie", 2016, 7.6, "Science Fiction, Drama", 210],
  ["Severance", "series", 2022, 8.4, "Drama, Mystery", 190],
  ["Interstellar", "movie", 2014, 8.4, "Adventure, Drama, Science Fiction", 230],
  ["Frieren: Beyond Journey's End", "series", 2023, 8.9, "Animation, Fantasy", 160],
  ["Casablanca", "movie", 1942, 8.2, "Drama, Romance", 40],
  ["Parasite", "movie", 2019, 8.5, "Comedy, Thriller, Drama", 95],
  ["Mad Max: Fury Road", "movie", 2015, 7.6, "Action, Adventure", 15],
  ["Cowboy Bebop", "series", 1998, 8.4, "Animation, Sci-Fi", 265],
] as const;

const items: MediaItem[] = T.map(([title, kind, year, rating, genres, hue], i) => ({
  id: i + 1,
  kind,
  title,
  year,
  category: kind === "movie" ? "Movies" : title === "Attack on Titan" || title.startsWith("Frieren") || title === "Cowboy Bebop" ? "Anime" : "TV",
  tmdb_id: 1000 + i,
  poster_path: i === 9 ? null : art(title, hue),
  overview:
    "A sweeping story of ambition and consequence, told across years and continents. Sample text for the design preview: real titles show their TMDb overview here.",
  rating,
  genres,
  tags: null,
  collections: null,
  episode_count: kind === "movie" ? 1 : 10,
  watched_count: i % 4 === 0 ? (kind === "movie" ? 1 : 10) : i % 3 === 0 ? 4 : 0,
  last_watched: null,
  added_at: `2026-10-0${(i % 8) + 1} 12:00:00`,
  total_size: 4_000_000_000,
}));

function episode(item: MediaItem, n: number, season = 1): Episode {
  return {
    id: item.id * 100 + season * 20 + n,
    media_item_id: item.id,
    path: `D:\\Media\\${item.title}\\${item.title} - S0${season}E0${n}.mkv`,
    file_name: `${item.title} - S0${season}E0${n}.mkv`,
    season: item.kind === "series" ? season : null,
    episode: item.kind === "series" ? n : null,
    size: 1_500_000_000,
    modified: 1_727_000_000,
    duration_secs: item.kind === "series" ? 3000 : 9200,
    title: item.kind === "series" ? ["Secrets", "Lies", "Past and Present", "Double Lives", "Truths", "Sic Mundus", "Crossroads", "As You Sow", "Everything Is Now", "Alpha and Omega"][n - 1] ?? `Episode ${n}` : null,
    overview: item.kind === "series" ? "A disappearance sets four families on a hunt for answers as they unearth a mind-bending mystery that spans generations." : null,
    air_date: "2017-12-01",
    still_path: item.kind === "series" ? art(`${item.title} ${n}`, ((item.id * 37 + n * 23) % 360), true) : null,
    rating: 7.9,
    extra: null,
    subtitles: null,
    position_secs: n === 4 ? 1400 : 0,
    completed: n < 4,
    last_watched: n <= 4 ? "2026-10-07 21:00:00" : null,
    available: true,
  };
}

function details(item: MediaItem): Details {
  const hue = (T[item.id - 1]?.[5] as number) ?? 120;
  return {
    tmdb_id: item.tmdb_id ?? 0,
    kind: item.kind,
    title: item.title,
    original_title: null,
    tagline: "Fear is the mind-killer.",
    overview: item.overview,
    genres: (item.genres ?? "").split(", "),
    runtime_min: item.kind === "movie" ? 166 : 52,
    rating: item.rating,
    vote_count: 5321,
    release_date: `${item.year}-03-01`,
    last_air_date: item.kind === "series" ? "2020-06-27" : null,
    status: item.kind === "series" ? "Ended" : "Released",
    certification: "PG-13",
    language: "en",
    backdrop_path: art(item.title, hue, true),
    cast: ["Timothée Chalamet", "Zendaya", "Rebecca Ferguson", "Javier Bardem", "Austin Butler", "Florence Pugh", "Josh Brolin", "Stellan Skarsgård"].map((name, i) => ({
      name,
      character: ["Paul Atreides", "Chani", "Lady Jessica", "Stilgar", "Feyd-Rautha", "Princess Irulan", "Gurney Halleck", "Baron Harkonnen"][i],
      photo_url: null,
    })),
    directors: ["Denis Villeneuve"],
    writers: ["Jon Spaihts", "Denis Villeneuve"],
    creators: item.kind === "series" ? ["Baran bo Odar", "Jantje Friese"] : [],
    companies: ["Legendary Pictures"],
    trailer_youtube: "abc",
    imdb_id: "tt15239678",
    homepage: null,
    tmdb_url: "https://www.themoviedb.org",
    number_of_seasons: item.kind === "series" ? 2 : null,
    number_of_episodes: item.kind === "series" ? 18 : null,
    seasons: item.kind === "series" ? [{ season_number: 1, name: "Season 1", episode_count: 10, air_date: "2017-12-01", overview: null, poster_url: null }] : [],
    fetched_at: "2026-10-08",
    collection_name: null,
  };
}

const continueItems: ContinueItem[] = [5, 1, 7, 12, 2].map((id) => {
  const it = items[id - 1];
  const ep = episode(it, it.kind === "series" ? 4 : 1);
  ep.position_secs = it.kind === "series" ? 1400 : 5100;
  ep.completed = false;
  return { episode: ep, title: it.title, kind: it.kind, year: it.year, poster_path: it.poster_path, tmdb_id: it.tmdb_id };
});

const collections: Tag[] = [
  { id: 1, name: "Dune Collection", kind: "collection", tmdb_collection_id: 1, poster_url: null, overview: null, item_count: 2, watched_count: 1, first_poster: items[0].poster_path, first_tmdb_id: 1000 },
  { id: 2, name: "Studio Ghibli", kind: "collection", tmdb_collection_id: null, poster_url: null, overview: null, item_count: 2, watched_count: 0, first_poster: items[8].poster_path, first_tmdb_id: 1008 },
];

export async function previewInvoke(cmd: string, args?: Record<string, unknown>): Promise<unknown> {
  const id = (args?.mediaItemId ?? args?.id) as number | undefined;
  const item = id ? items.find((m) => m.id === id) : undefined;
  switch (cmd) {
    case "list_media": return args?.kind ? items.filter((m) => m.kind === args.kind) : items;
    case "continue_watching": return continueItems;
    case "list_tags": return args?.kind === "collection" ? collections : [];
    case "collection_items": return items.slice(0, 2);
    case "get_media_item": return item ?? null;
    case "list_episodes": return item ? (item.kind === "series" ? Array.from({ length: 8 }, (_, i) => episode(item, i + 1)) : [episode(item, 1)]) : [];
    case "get_details": return item ? details(item) : null;
    case "find_duplicates": return [];
    case "get_settings": return {};
    case "list_categories": return ["Movies", "TV", "Anime"];
    case "search": return { items: items.filter((m) => m.title.toLowerCase().includes(String(args?.query ?? "").toLowerCase())), episodes: [] };
    case "list_libraries": return [{ id: 1, path: "D:\\Media", name: "Media", available: true }];
    case "tmdb_store": return { titles: 16, details: 16, images: 30, image_bytes: 48_000_000 };
    case "get_tmdb_status": return "••••••••a1b2";
    case "detect_players": case "list_drives": case "list_history": case "pending_open_urls": case "default_ignored_dirs": case "torrent_list": return [];
    case "rename_can_undo": return false;
    default: return null;
  }
}
