use crate::db::{self, MediaItem};
use crate::AppState;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

const API: &str = "https://api.themoviedb.org/3";
const IMG: &str = "https://image.tmdb.org/t/p/w342";

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TmdbMatch {
    pub tmdb_id: i64,
    pub title: String,
    pub year: Option<i32>,
    pub overview: Option<String>,
    /// TMDB relative path like "/abc.jpg"
    pub poster: Option<String>,
    pub poster_url: Option<String>,
    #[serde(default)]
    pub rating: Option<f64>,
    #[serde(default)]
    pub genre_ids: Vec<i64>,
}

#[derive(Deserialize)]
struct SearchResponse {
    results: Vec<SearchResult>,
}

#[derive(Deserialize)]
struct SearchResult {
    id: i64,
    title: Option<String>,
    name: Option<String>,
    release_date: Option<String>,
    first_air_date: Option<String>,
    overview: Option<String>,
    poster_path: Option<String>,
    vote_average: Option<f64>,
    #[serde(default)]
    genre_ids: Vec<i64>,
}

/// Pinned so genre names, overviews and episode titles are always English
/// regardless of TMDB's locale guess.
const LANG: &str = "en-US";

static GENRE_CACHE: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<String, std::collections::HashMap<i64, String>>>> =
    std::sync::OnceLock::new();

/// Map TMDB genre ids to names, fetching the list for this kind once per run.
fn genre_names(key: &str, kind: &str, ids: &[i64]) -> Option<String> {
    if ids.is_empty() {
        return None;
    }
    let cache = GENRE_CACHE.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()));
    let kind_key = if kind == "movie" { "movie" } else { "tv" };
    {
        let map = cache.lock().ok()?;
        if let Some(m) = map.get(kind_key) {
            return join_genres(m, ids);
        }
    }
    #[derive(Deserialize)]
    struct G {
        id: i64,
        name: String,
    }
    #[derive(Deserialize)]
    struct GL {
        genres: Vec<G>,
    }
    let req = client().get(format!("{API}/genre/{kind_key}/list")).query(&[("language", LANG)]);
    let list: GL = auth(req, key).send().ok()?.json().ok()?;
    let m: std::collections::HashMap<i64, String> = list.genres.into_iter().map(|g| (g.id, g.name)).collect();
    let out = join_genres(&m, ids);
    cache.lock().ok()?.insert(kind_key.to_string(), m);
    out
}

fn join_genres(m: &std::collections::HashMap<i64, String>, ids: &[i64]) -> Option<String> {
    let names: Vec<&str> = ids.iter().filter_map(|i| m.get(i).map(|s| s.as_str())).collect();
    (!names.is_empty()).then(|| names.join(", "))
}

fn client() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(20))
        .user_agent("vortex-media-manager")
        .build()
        .expect("http client")
}

fn auth(req: reqwest::blocking::RequestBuilder, key: &str) -> reqwest::blocking::RequestBuilder {
    // v4 read-access tokens are JWTs and go in the header; v3 keys go in the query.
    if key.starts_with("eyJ") {
        req.bearer_auth(key)
    } else {
        req.query(&[("api_key", key)])
    }
}

pub fn search(key: &str, kind: &str, query: &str, year: Option<i32>) -> Result<Vec<TmdbMatch>, String> {
    let endpoint = if kind == "movie" { "search/movie" } else { "search/tv" };
    let year_param = if kind == "movie" { "year" } else { "first_air_date_year" };
    let mut req = client()
        .get(format!("{API}/{endpoint}"))
        .query(&[("query", query), ("include_adult", "false"), ("language", LANG)]);
    if let Some(y) = year {
        req = req.query(&[(year_param, y.to_string())]);
    }
    let resp = auth(req, key).send().map_err(|e| format!("TMDB request failed: {e}"))?;
    let status = resp.status();
    if status.as_u16() == 401 {
        return Err("TMDB rejected the API key".into());
    }
    if !status.is_success() {
        return Err(format!("TMDB returned HTTP {status}"));
    }
    let body: SearchResponse = resp.json().map_err(|e| format!("bad TMDB response: {e}"))?;
    Ok(body
        .results
        .into_iter()
        .map(|r| {
            let date = r.release_date.or(r.first_air_date).unwrap_or_default();
            TmdbMatch {
                tmdb_id: r.id,
                title: r.title.or(r.name).unwrap_or_default(),
                year: date.get(..4).and_then(|y| y.parse().ok()),
                overview: r.overview.filter(|o| !o.is_empty()),
                poster_url: r.poster_path.as_ref().map(|p| format!("{IMG}{p}")),
                poster: r.poster_path,
                rating: r.vote_average.filter(|v| *v > 0.0).map(|v| (v * 10.0).round() / 10.0),
                genre_ids: r.genre_ids,
            }
        })
        .collect())
}

/// Search with the year first; if nothing comes back, retry without it.
fn best_match(key: &str, item: &MediaItem) -> Result<Option<TmdbMatch>, String> {
    let with_year = search(key, &item.kind, &item.title, item.year)?;
    if let Some(m) = with_year.into_iter().next() {
        return Ok(Some(m));
    }
    if item.year.is_some() {
        return Ok(search(key, &item.kind, &item.title, None)?.into_iter().next());
    }
    Ok(None)
}

#[derive(Deserialize)]
struct SeasonResponse {
    episodes: Vec<SeasonEpisode>,
}

#[derive(Deserialize)]
struct SeasonEpisode {
    episode_number: i32,
    name: Option<String>,
    overview: Option<String>,
    air_date: Option<String>,
    still_path: Option<String>,
    vote_average: Option<f64>,
}

/// Fetch episode names for every season present in the library for this
/// series. Returns how many episodes were updated.
pub fn fetch_episode_titles(app: &AppHandle, media_item_id: i64) -> Result<usize, String> {
    let state = app.state::<AppState>();
    let (key, tmdb_id, seasons) = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let key = db::get_setting(&conn, "tmdb_key").map_err(|e| e.to_string())?.unwrap_or_default();
        let item = db::get_media_item(&conn, media_item_id).map_err(|e| e.to_string())?.ok_or("item not found")?;
        if item.kind != "series" {
            return Ok(0);
        }
        let tmdb_id = item.tmdb_id.ok_or("Match this series to TMDB first (Fix match)")?;
        (key, tmdb_id, db::seasons_for_item(&conn, media_item_id).map_err(|e| e.to_string())?)
    };
    if key.trim().is_empty() {
        return Err("Add your TMDB API key in Settings first".into());
    }
    let mut updated = 0;
    for season in seasons {
        let req = client().get(format!("{API}/tv/{tmdb_id}/season/{season}")).query(&[("language", LANG)]);
        let resp = auth(req, &key).send().map_err(|e| format!("TMDB request failed: {e}"))?;
        if resp.status().as_u16() == 404 {
            continue; // season numbering differs from TMDB; skip it
        }
        if !resp.status().is_success() {
            return Err(format!("TMDB returned HTTP {}", resp.status()));
        }
        let body: SeasonResponse = resp.json().map_err(|e| format!("bad TMDB response: {e}"))?;
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        for ep in body.episodes {
            updated += db::set_episode_meta(
                &conn,
                media_item_id,
                season,
                ep.episode_number,
                ep.name.as_deref().filter(|s| !s.is_empty()),
                ep.overview.as_deref().filter(|s| !s.is_empty()),
                ep.air_date.as_deref().filter(|s| !s.is_empty()),
                ep.still_path.as_deref().map(|p| format!("https://image.tmdb.org/t/p/w300{p}")).as_deref(),
                ep.vote_average.filter(|v| *v > 0.0),
            )
            .map_err(|e| e.to_string())?;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    Ok(updated)
}

// ---------- full details (IMDb-style page) ----------

#[derive(Serialize, Clone, Debug)]
pub struct CastMember {
    pub name: String,
    pub character: Option<String>,
    pub photo_url: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
pub struct SeasonInfo {
    pub season_number: i32,
    pub name: String,
    pub episode_count: i32,
    pub air_date: Option<String>,
    pub overview: Option<String>,
    pub poster_url: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
pub struct Details {
    pub tmdb_id: i64,
    pub kind: String,
    pub title: String,
    pub original_title: Option<String>,
    pub tagline: Option<String>,
    pub overview: Option<String>,
    pub genres: Vec<String>,
    pub runtime_min: Option<i32>,
    pub rating: Option<f64>,
    pub vote_count: Option<i64>,
    pub release_date: Option<String>,
    pub last_air_date: Option<String>,
    pub status: Option<String>,
    pub certification: Option<String>,
    pub language: Option<String>,
    pub backdrop_path: Option<String>,
    pub cast: Vec<CastMember>,
    pub directors: Vec<String>,
    pub writers: Vec<String>,
    pub creators: Vec<String>,
    pub companies: Vec<String>,
    pub trailer_youtube: Option<String>,
    pub imdb_id: Option<String>,
    pub homepage: Option<String>,
    pub tmdb_url: String,
    pub number_of_seasons: Option<i32>,
    pub number_of_episodes: Option<i32>,
    pub seasons: Vec<SeasonInfo>,
    pub fetched_at: String,
    pub collection_name: Option<String>,
}

#[derive(Deserialize, Default)]
struct RawNamed {
    name: Option<String>,
}
#[derive(Deserialize, Default)]
struct RawCast {
    name: Option<String>,
    character: Option<String>,
    profile_path: Option<String>,
    order: Option<i32>,
}
#[derive(Deserialize, Default)]
struct RawCrew {
    name: Option<String>,
    job: Option<String>,
    department: Option<String>,
}
#[derive(Deserialize, Default)]
struct RawCredits {
    #[serde(default)]
    cast: Vec<RawCast>,
    #[serde(default)]
    crew: Vec<RawCrew>,
}
#[derive(Deserialize, Default)]
struct RawVideo {
    key: Option<String>,
    site: Option<String>,
    #[serde(rename = "type")]
    kind: Option<String>,
    official: Option<bool>,
}
#[derive(Deserialize, Default)]
struct RawVideos {
    #[serde(default)]
    results: Vec<RawVideo>,
}
#[derive(Deserialize, Default)]
struct RawExternal {
    imdb_id: Option<String>,
}
#[derive(Deserialize, Default)]
struct RawRelease {
    certification: Option<String>,
}
#[derive(Deserialize, Default)]
struct RawReleaseCountry {
    iso_3166_1: Option<String>,
    #[serde(default)]
    release_dates: Vec<RawRelease>,
    rating: Option<String>,
}
#[derive(Deserialize, Default)]
struct RawReleases {
    #[serde(default)]
    results: Vec<RawReleaseCountry>,
}
#[derive(Deserialize, Default)]
struct RawSeason {
    season_number: Option<i32>,
    name: Option<String>,
    episode_count: Option<i32>,
    air_date: Option<String>,
    overview: Option<String>,
    poster_path: Option<String>,
}
#[derive(Deserialize, Default)]
struct RawDetails {
    id: Option<i64>,
    title: Option<String>,
    name: Option<String>,
    original_title: Option<String>,
    original_name: Option<String>,
    tagline: Option<String>,
    overview: Option<String>,
    #[serde(default)]
    genres: Vec<RawNamed>,
    runtime: Option<i32>,
    #[serde(default)]
    episode_run_time: Vec<i32>,
    vote_average: Option<f64>,
    vote_count: Option<i64>,
    release_date: Option<String>,
    first_air_date: Option<String>,
    last_air_date: Option<String>,
    status: Option<String>,
    original_language: Option<String>,
    backdrop_path: Option<String>,
    homepage: Option<String>,
    number_of_seasons: Option<i32>,
    number_of_episodes: Option<i32>,
    #[serde(default)]
    production_companies: Vec<RawNamed>,
    #[serde(default)]
    networks: Vec<RawNamed>,
    #[serde(default)]
    created_by: Vec<RawNamed>,
    #[serde(default)]
    seasons: Vec<RawSeason>,
    #[serde(default)]
    credits: RawCredits,
    #[serde(default)]
    videos: RawVideos,
    #[serde(default)]
    external_ids: RawExternal,
    #[serde(default)]
    release_dates: RawReleases,
    #[serde(default)]
    content_ratings: RawReleases,
    belongs_to_collection: Option<RawCollection>,
}

#[derive(Deserialize, Default)]
struct RawCollection {
    id: Option<i64>,
    name: Option<String>,
    poster_path: Option<String>,
}

fn non_empty(s: Option<String>) -> Option<String> {
    s.filter(|v| !v.trim().is_empty())
}

fn names(v: Vec<RawNamed>) -> Vec<String> {
    v.into_iter().filter_map(|n| non_empty(n.name)).collect()
}

fn dedupe(v: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for s in v {
        if !out.contains(&s) {
            out.push(s);
        }
    }
    out
}

fn certification(kind: &str, raw: &RawDetails) -> Option<String> {
    let list = if kind == "movie" { &raw.release_dates.results } else { &raw.content_ratings.results };
    let pick = |c: &RawReleaseCountry| -> Option<String> {
        if kind == "movie" {
            c.release_dates.iter().find_map(|r| non_empty(r.certification.clone()))
        } else {
            non_empty(c.rating.clone())
        }
    };
    list.iter()
        .find(|c| c.iso_3166_1.as_deref() == Some("US"))
        .and_then(pick)
        .or_else(|| list.iter().find_map(pick))
}

fn normalize(kind: &str, raw: RawDetails, backdrop_local: Option<String>, fetched_at: String) -> Details {
    let tmdb_id = raw.id.unwrap_or(0);
    let mut cast: Vec<RawCast> = raw.credits.cast;
    cast.sort_by_key(|c| c.order.unwrap_or(999));
    let cast = cast
        .into_iter()
        .take(24)
        .filter_map(|c| {
            Some(CastMember {
                name: non_empty(c.name)?,
                character: non_empty(c.character),
                photo_url: c.profile_path.map(|p| format!("https://image.tmdb.org/t/p/w185{p}")),
            })
        })
        .collect();
    let directors = dedupe(
        raw.credits.crew.iter().filter(|c| c.job.as_deref() == Some("Director")).filter_map(|c| non_empty(c.name.clone())).collect(),
    );
    let writers = dedupe(
        raw.credits.crew.iter().filter(|c| c.department.as_deref() == Some("Writing")).filter_map(|c| non_empty(c.name.clone())).collect(),
    );
    let trailer = raw
        .videos
        .results
        .iter()
        .filter(|v| v.site.as_deref() == Some("YouTube") && v.kind.as_deref() == Some("Trailer"))
        .max_by_key(|v| v.official.unwrap_or(false))
        .and_then(|v| v.key.clone());
    let runtime = if kind == "movie" { raw.runtime } else { raw.episode_run_time.first().copied() }.filter(|r| *r > 0);
    let mut companies = names(raw.networks);
    companies.extend(names(raw.production_companies));
    let seasons = raw
        .seasons
        .into_iter()
        .filter(|s| s.season_number.unwrap_or(0) > 0)
        .map(|s| SeasonInfo {
            season_number: s.season_number.unwrap_or(0),
            name: s.name.unwrap_or_default(),
            episode_count: s.episode_count.unwrap_or(0),
            air_date: non_empty(s.air_date),
            overview: non_empty(s.overview),
            poster_url: s.poster_path.map(|p| format!("https://image.tmdb.org/t/p/w185{p}")),
        })
        .collect();
    Details {
        tmdb_id,
        kind: kind.to_string(),
        title: raw.title.or(raw.name).unwrap_or_default(),
        original_title: non_empty(raw.original_title.or(raw.original_name)),
        tagline: non_empty(raw.tagline),
        overview: non_empty(raw.overview),
        genres: names(raw.genres),
        runtime_min: runtime,
        rating: raw.vote_average.filter(|v| *v > 0.0).map(|v| (v * 10.0).round() / 10.0),
        vote_count: raw.vote_count,
        release_date: non_empty(raw.release_date.or(raw.first_air_date)),
        last_air_date: non_empty(raw.last_air_date),
        status: non_empty(raw.status),
        certification: certification(kind, &RawDetails { release_dates: raw.release_dates, content_ratings: raw.content_ratings, ..Default::default() }),
        language: non_empty(raw.original_language),
        backdrop_path: backdrop_local,
        cast,
        directors,
        writers,
        creators: names(raw.created_by),
        companies: dedupe(companies).into_iter().take(4).collect(),
        trailer_youtube: trailer,
        imdb_id: non_empty(raw.external_ids.imdb_id),
        homepage: non_empty(raw.homepage),
        tmdb_url: format!("https://www.themoviedb.org/{}/{tmdb_id}", if kind == "movie" { "movie" } else { "tv" }),
        number_of_seasons: raw.number_of_seasons,
        number_of_episodes: raw.number_of_episodes,
        seasons,
        fetched_at,
        collection_name: raw.belongs_to_collection.and_then(|c| c.name),
    }
}

fn backdrop_file(app: &AppHandle, media_item_id: i64, raw_path: Option<&str>) -> Option<String> {
    let dest = posters_dir(app).ok()?.join(format!("{media_item_id}_backdrop.jpg"));
    if dest.exists() {
        return Some(dest.to_string_lossy().to_string());
    }
    let p = raw_path?;
    let bytes = client()
        .get(format!("https://image.tmdb.org/t/p/w1280{p}"))
        .send()
        .ok()?
        .error_for_status()
        .ok()?
        .bytes()
        .ok()?;
    std::fs::write(&dest, &bytes).ok()?;
    Some(dest.to_string_lossy().to_string())
}

/// Full details for the detail page. Served from the local cache unless
/// `refresh` is set or the cache is older than 30 days.
pub fn get_details(app: &AppHandle, media_item_id: i64, refresh: bool) -> Result<Option<Details>, String> {
    let state = app.state::<AppState>();
    let (key, kind, tmdb_id, cached) = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let item = db::get_media_item(&conn, media_item_id).map_err(|e| e.to_string())?.ok_or("item not found")?;
        let Some(tmdb_id) = item.tmdb_id else { return Ok(None) };
        let key = db::get_setting(&conn, "tmdb_key").map_err(|e| e.to_string())?.unwrap_or_default();
        let cached = db::get_details_json(&conn, media_item_id).map_err(|e| e.to_string())?;
        (key, item.kind, tmdb_id, cached)
    };

    let stale = |fetched_at: &str| -> bool {
        // fetched_at is "YYYY-MM-DD HH:MM:SS" UTC; compare on the date part, 30 days.
        let days = |s: &str| -> Option<i64> {
            let d: Vec<i64> = s.get(..10)?.split('-').filter_map(|p| p.parse().ok()).collect();
            if d.len() != 3 { return None; }
            Some(d[0] * 372 + d[1] * 31 + d[2])
        };
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        let today = chrono_free_date(now);
        match (days(fetched_at), days(&today)) {
            (Some(a), Some(b)) => b - a > 30,
            _ => true,
        }
    };

    if let (false, Some((cached_id, json, fetched_at))) = (refresh, &cached) {
        if *cached_id == tmdb_id && !stale(fetched_at) {
            let raw: RawDetails = serde_json::from_str(json).map_err(|e| e.to_string())?;
            let backdrop = backdrop_file(app, media_item_id, raw.backdrop_path.as_deref());
            return Ok(Some(normalize(&kind, raw, backdrop, fetched_at.clone())));
        }
    }
    if key.trim().is_empty() {
        // Offline fallback: serve stale cache rather than nothing.
        if let Some((_, json, fetched_at)) = cached {
            let raw: RawDetails = serde_json::from_str(&json).map_err(|e| e.to_string())?;
            let backdrop = backdrop_file(app, media_item_id, raw.backdrop_path.as_deref());
            return Ok(Some(normalize(&kind, raw, backdrop, fetched_at)));
        }
        return Err("Add your TMDB API key in Settings first".into());
    }

    let (path, append) = if kind == "movie" {
        (format!("movie/{tmdb_id}"), "credits,videos,external_ids,release_dates")
    } else {
        (format!("tv/{tmdb_id}"), "credits,videos,external_ids,content_ratings")
    };
    let req = client().get(format!("{API}/{path}")).query(&[("append_to_response", append), ("language", LANG)]);
    let resp = auth(req, &key).send().map_err(|e| format!("TMDB request failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("TMDB returned HTTP {}", resp.status()));
    }
    let json = resp.text().map_err(|e| e.to_string())?;
    let raw: RawDetails = serde_json::from_str(&json).map_err(|e| format!("bad TMDB response: {e}"))?;
    {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        db::set_details_json(&conn, media_item_id, tmdb_id, &json).map_err(|e| e.to_string())?;
        let rating = raw.vote_average.filter(|v| *v > 0.0).map(|v| (v * 10.0).round() / 10.0);
        db::set_item_rating(&conn, media_item_id, rating).map_err(|e| e.to_string())?;
        let genres: Vec<String> = raw.genres.iter().filter_map(|g| g.name.clone()).filter(|n| !n.is_empty()).collect();
        db::set_item_genres(&conn, media_item_id, (!genres.is_empty()).then(|| genres.join(", ")).as_deref())
            .map_err(|e| e.to_string())?;
        if let Some(c) = &raw.belongs_to_collection {
            if let (Some(cid), Some(name)) = (c.id, c.name.clone()) {
                let url = c.poster_path.as_ref().map(|p| format!("{IMG}{p}"));
                let _ = db::attach_tmdb_collection(&conn, media_item_id, cid, &name, url.as_deref());
            }
        }
    }
    // A refresh should pick up a changed backdrop too.
    if refresh {
        if let Ok(dir) = posters_dir(app) {
            let _ = std::fs::remove_file(dir.join(format!("{media_item_id}_backdrop.jpg")));
        }
    }
    let backdrop = backdrop_file(app, media_item_id, raw.backdrop_path.as_deref());
    let fetched_at = chrono_free_date(
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0),
    );
    Ok(Some(normalize(&kind, raw, backdrop, fetched_at)))
}

pub fn today() -> String {
    chrono_free_date(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0))
}

/// "YYYY-MM-DD" from a unix timestamp without pulling in a date crate.
fn chrono_free_date(secs: u64) -> String {
    let days = (secs / 86400) as i64;
    // Civil-from-days (Howard Hinnant's algorithm).
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

pub fn posters_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?.join("posters");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn download_poster(poster: &str, dest: &Path) -> Result<(), String> {
    let bytes = client()
        .get(format!("{IMG}{poster}"))
        .send()
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .bytes()
        .map_err(|e| e.to_string())?;
    std::fs::write(dest, &bytes).map_err(|e| e.to_string())
}

/// Apply a chosen match to an item: download the poster and store metadata.
pub fn apply_match(app: &AppHandle, media_item_id: i64, m: &TmdbMatch) -> Result<(), String> {
    let mut poster_file: Option<String> = None;
    if let Some(p) = &m.poster {
        let dest = posters_dir(app)?.join(format!("{media_item_id}.jpg"));
        download_poster(p, &dest)?;
        poster_file = Some(dest.to_string_lossy().to_string());
    }
    let state = app.state::<AppState>();
    {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        db::set_tmdb(&conn, media_item_id, Some(m.tmdb_id), poster_file.as_deref(), m.overview.as_deref(), m.rating)
            .map_err(|e| e.to_string())?;
        let kind: String = conn
            .query_row("SELECT kind FROM media_items WHERE id = ?1", [media_item_id], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        let key = db::get_setting(&conn, "tmdb_key").map_err(|e| e.to_string())?.unwrap_or_default();
        if let Some(g) = genre_names(&key, &kind, &m.genre_ids) {
            let _ = db::set_item_genres(&conn, media_item_id, Some(&g));
        }
    }
    // Episode names and the detail page are nice-to-haves; a failure here must not undo the match.
    let _ = fetch_episode_titles(app, media_item_id);
    let _ = get_details(app, media_item_id, true);
    Ok(())
}

#[derive(Serialize, Clone)]
struct Progress {
    done: usize,
    total: usize,
    matched: usize,
    current: String,
}

/// Background job: find posters for every item that has none.
/// `force` also retries items that were checked before and found nothing.
pub fn fetch_missing(app: AppHandle, force: bool) -> Result<(), String> {
    let state = app.state::<AppState>();
    if state.fetching.swap(true, Ordering::SeqCst) {
        return Err("A poster fetch is already running".into());
    }
    let (key, items) = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let key = db::get_setting(&conn, "tmdb_key").map_err(|e| e.to_string())?.unwrap_or_default();
        let items = db::items_missing_poster(&conn, force).map_err(|e| e.to_string())?;
        (key, items)
    };
    if key.trim().is_empty() {
        state.fetching.store(false, Ordering::SeqCst);
        return Err("Add your TMDB API key in Settings first".into());
    }

    std::thread::spawn(move || {
        let total = items.len();
        let mut matched = 0;
        for (i, item) in items.iter().enumerate() {
            let _ = app.emit("posters-progress", Progress { done: i, total, matched, current: item.title.clone() });
            match best_match(&key, item) {
                Ok(Some(m)) => {
                    if apply_match(&app, item.id, &m).is_ok() {
                        matched += 1;
                    }
                }
                Ok(None) => {
                    let state = app.state::<AppState>();
                    let guard = state.db.lock();
                    if let Ok(conn) = guard {
                        let _ = db::mark_poster_checked(&conn, item.id);
                    }
                }
                Err(e) => {
                    // Auth or network failure: stop the whole run rather than hammer the API.
                    let _ = app.emit("posters-done", Progress { done: i, total, matched, current: e });
                    app.state::<AppState>().fetching.store(false, Ordering::SeqCst);
                    return;
                }
            }
            std::thread::sleep(Duration::from_millis(120)); // stay well under TMDB's rate limit
        }
        let _ = app.emit("posters-done", Progress { done: total, total, matched, current: String::new() });
        app.state::<AppState>().fetching.store(false, Ordering::SeqCst);
    });
    Ok(())
}
