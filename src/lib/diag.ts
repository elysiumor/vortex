/**
 * Renderer-side diagnostics, forwarded into the Rust log.
 *
 * The backend log cannot see the WebView, which is where scroll jank, slow
 * IPC and uncaught errors actually happen. This module reports those three
 * things so a "the app is slow" report arrives with numbers attached.
 */
import { invoke } from "@tauri-apps/api/core";

/** Anything slower than this blocks a frame badly enough to be felt. */
const SLOW_CALL_MS = 250;
const LONG_TASK_MS = 200;

function send(level: "info" | "warn" | "error", message: string) {
  // Never await, and never let logging throw into the caller's path.
  invoke("log_frontend", { level, message }).catch(() => {});
}

/** Wraps `invoke` so any slow command lands in the log with its duration. */
export async function timedInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const started = performance.now();
  try {
    return await invoke<T>(cmd, args);
  } catch (e) {
    send("error", `command ${cmd} failed after ${Math.round(performance.now() - started)}ms: ${e}`);
    throw e;
  } finally {
    const ms = Math.round(performance.now() - started);
    if (ms >= SLOW_CALL_MS) send("warn", `slow command ${cmd} took ${ms}ms`);
  }
}

let installed = false;

export function installDiagnostics() {
  if (installed) return;
  installed = true;

  window.addEventListener("error", (e) => {
    send("error", `uncaught ${e.message} at ${e.filename}:${e.lineno}:${e.colno}`);
  });
  window.addEventListener("unhandledrejection", (e) => {
    send("error", `unhandled rejection: ${e.reason}`);
  });

  // A long task is the renderer being unable to paint. This is what "stuck
  // while scrolling" looks like from the inside.
  try {
    new PerformanceObserver((list) => {
      for (const entry of list.getEntries()) {
        if (entry.duration >= LONG_TASK_MS) {
          send("warn", `UI blocked for ${Math.round(entry.duration)}ms (${entry.name})`);
        }
      }
    }).observe({ entryTypes: ["longtask"] });
  } catch {
    // Older WebView2 builds lack the longtask entry type; not fatal.
  }

  // Frame-rate collapse during scrolling, sampled rather than logged per frame.
  let frames = 0;
  let windowStart = performance.now();
  let worstGap = 0;
  let lastFrame = windowStart;
  const tick = () => {
    const now = performance.now();
    worstGap = Math.max(worstGap, now - lastFrame);
    lastFrame = now;
    frames++;
    if (now - windowStart >= 5000) {
      const fps = (frames * 1000) / (now - windowStart);
      // Only complain when it is genuinely bad and the window was active.
      if (fps < 30 && worstGap > 100 && !document.hidden) {
        send("warn", `render stalled: ${fps.toFixed(0)} fps over 5s, worst frame gap ${Math.round(worstGap)}ms`);
      }
      frames = 0;
      worstGap = 0;
      windowStart = now;
    }
    requestAnimationFrame(tick);
  };
  requestAnimationFrame(tick);

  send("info", `UI ready: ${window.innerWidth}x${window.innerHeight}, dpr ${window.devicePixelRatio}`);
}
