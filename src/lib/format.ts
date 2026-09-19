import type { Episode } from "./api";

export function hms(secs: number): string {
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  const s = Math.floor(secs % 60);
  const mm = String(m).padStart(2, "0");
  const ss = String(s).padStart(2, "0");
  return h > 0 ? `${h}:${mm}:${ss}` : `${m}:${ss}`;
}

export function parseHms(input: string): number | null {
  const parts = input.trim().split(":").map((p) => p.trim());
  if (parts.some((p) => p === "" || !/^\d+$/.test(p))) return null;
  const nums = parts.map(Number);
  if (nums.length === 1) return nums[0] * 60; // bare minutes
  if (nums.length === 2) return nums[0] * 60 + nums[1];
  if (nums.length === 3) return nums[0] * 3600 + nums[1] * 60 + nums[2];
  return null;
}

export function episodeCode(ep: Episode): string {
  if (ep.season != null && ep.episode != null) {
    return `S${String(ep.season).padStart(2, "0")}E${String(ep.episode).padStart(2, "0")}`;
  }
  if (ep.episode != null) return `E${String(ep.episode).padStart(2, "0")}`;
  return "";
}

const JUNK = /\b(2160p|1080p|1080i|720p|480p|4k|uhd|hdr10?|dv|x26[45]|h\.?26[45]|hevc|avc|xvid|10bit|bluray|bdrip|brrip|webrip|web-?dl|web|hdrip|dvdrip|hdtv|remux|aac|ac3|dts|ddp?5\.?1|atmos|truehd|yify|yts|rarbg|proper|repack|extended|remastered|internal|amzn|nf|dsnp|hmax|atvp)\b.*$/i;

/** TMDB title when we have one, otherwise a cleaned-up filename. */
export function episodeTitle(ep: Episode): string {
  return ep.title || prettyName(ep);
}

/** Human-readable episode title from a release filename, e.g.
 *  "Show.S01E03.You.Will.Gladden.1080p.NF.WEB-DL.mkv" -> "You Will Gladden". */
export function prettyName(ep: Episode): string {
  let s = ep.file_name.replace(/\.[^.]+$/, "");
  const m = /(?:S\d{1,2}[ ._-]?E\d{1,3}(?:[ ._-]?E?\d{1,3})*|\d{1,2}x\d{1,3})[ ._-]*/i.exec(s);
  if (m) s = s.slice(m.index + m[0].length);
  s = s.replace(/[._]/g, " ").replace(JUNK, "").replace(/-\s*\w+$/, "").replace(/\s+/g, " ").trim();
  s = s.replace(/^[-–\s]+|[-–\s]+$/g, "");
  return s.length >= 2 ? s : ep.file_name;
}

export function folderOf(path: string): string {
  const i = Math.max(path.lastIndexOf("\\"), path.lastIndexOf("/"));
  return i > 0 ? path.slice(0, i) : path;
}

export function dateFromUnix(secs: number): string {
  if (!secs) return "unknown";
  return new Date(secs * 1000).toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" });
}

export function fileSizeExact(bytes: number): string {
  return `${fileSize(bytes)} (${bytes.toLocaleString()} bytes)`;
}

export async function copyText(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    return false;
  }
}

export function fileSize(bytes: number): string {
  if (bytes >= 1 << 30) return `${(bytes / (1 << 30)).toFixed(1)} GB`;
  return `${Math.round(bytes / (1 << 20))} MB`;
}

export function relativeTime(iso: string | null): string {
  if (!iso) return "";
  const then = new Date(iso.replace(" ", "T") + "Z").getTime();
  const diff = Math.max(0, Date.now() - then) / 1000;
  if (diff < 60) return "just now";
  if (diff < 3600) return `${Math.floor(diff / 60)} min ago`;
  if (diff < 86400) return `${Math.floor(diff / 3600)} h ago`;
  const d = Math.floor(diff / 86400);
  return d === 1 ? "yesterday" : `${d} days ago`;
}
