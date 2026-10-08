<script setup lang="ts">
import { computed, ref, onMounted, onUnmounted } from "vue";
import { Play, Check, X, Star } from "@lucide/vue";

const props = defineProps<{
  title: string;
  poster: string | null;
  subtitle?: string;
  meta?: string;
  /** 0..1 progress bar along the bottom of the poster */
  progress?: number | null;
  done?: boolean;
  badge?: string;
  /** TMDb rating out of 10, shown as a badge on the poster */
  rating?: number | null;
  dismissable?: boolean;
  playable?: boolean;
  unavailable?: boolean;
}>();
const emit = defineEmits<{ open: []; play: []; dismiss: [] }>();

const el = ref<HTMLElement>();
const initial = computed(() => props.title.trim().slice(0, 1).toUpperCase() || "?");
const onPlayEvent = () => emit("play");
onMounted(() => el.value?.addEventListener("play", onPlayEvent));
onUnmounted(() => el.value?.removeEventListener("play", onPlayEvent));
</script>

<template>
  <div
    ref="el"
    data-card
    tabindex="0"
    class="group relative flex cursor-pointer flex-col outline-none"
    :class="{ 'opacity-60': unavailable }"
    @click="emit('open')"
    @dblclick="playable && emit('play')"
  >
    <div class="card-shadow relative aspect-[2/3] overflow-hidden rounded-xl bg-muted ring-1 ring-black/5 transition-all duration-300 ease-out
                group-hover:-translate-y-1.5 group-hover:card-shadow-hover group-focus-visible:-translate-y-1.5 group-focus-visible:ring-2 group-focus-visible:ring-ring dark:ring-white/[0.06]">
      <img v-if="poster" :src="poster" :alt="title" loading="lazy" draggable="false"
           class="h-full w-full object-cover transition-transform duration-500 ease-out group-hover:scale-[1.06]" />
      <div v-else class="flex h-full w-full flex-col justify-end bg-gradient-to-br from-secondary via-muted to-accent p-4">
        <div class="text-5xl font-black text-foreground/15">{{ initial }}</div>
        <div class="mt-1 line-clamp-3 text-sm font-semibold leading-tight text-foreground/70">{{ title }}</div>
      </div>

      <div v-if="rating && !dismissable" class="glass-dark absolute right-2 top-2 flex items-center gap-1 rounded-full px-2 py-0.5 text-[11px] font-semibold">
        <Star class="size-3 fill-[var(--brand-yellow)] text-[var(--brand-yellow)]" />{{ rating.toFixed(1) }}
      </div>
      <div v-if="done" class="bg-brand absolute left-2 top-2 grid size-6 place-items-center rounded-full shadow-lg" title="Watched">
        <Check class="size-3.5" stroke-width="3" />
      </div>
      <button v-if="dismissable"
              class="glass-dark absolute right-2 top-2 grid size-7 place-items-center rounded-full opacity-0 transition-opacity hover:bg-destructive group-hover:opacity-100 group-focus-within:opacity-100"
              title="Remove from Continue watching (keeps progress)" @click.stop="emit('dismiss')">
        <X class="size-3.5" />
      </button>

      <div class="absolute inset-0 flex items-end bg-gradient-to-t from-black/85 via-black/20 to-transparent p-3 opacity-0 transition-opacity duration-300 group-hover:opacity-100 group-focus-visible:opacity-100">
        <button v-if="playable" :disabled="unavailable"
                class="bg-brand flex h-9 items-center gap-1.5 rounded-full pl-3 pr-4 text-sm font-semibold shadow-lg transition-transform hover:scale-105 disabled:opacity-60"
                @click.stop="emit('play')" @dblclick.stop>
          <Play class="size-4 fill-current" /> {{ unavailable ? "Offline" : "Play" }}
        </button>
        <span v-else class="text-xs font-medium text-white/80">Open</span>
      </div>

      <div v-if="progress != null && progress > 0" class="absolute inset-x-2 bottom-2 h-1 overflow-hidden rounded-full bg-white/25 group-hover:opacity-0">
        <div class="bg-brand h-full rounded-full" :style="{ width: Math.min(100, progress * 100) + '%' }"></div>
      </div>
    </div>

    <div class="mt-2.5 min-w-0 px-0.5">
      <div class="truncate text-[13.5px] font-semibold leading-snug">{{ title }}</div>
      <div v-if="subtitle" class="truncate text-xs text-muted-foreground">{{ subtitle }}</div>
      <div v-if="meta" class="truncate text-xs font-medium text-primary">{{ meta }}</div>
      <div v-if="badge" class="mt-1 w-fit rounded-full bg-secondary px-2 py-0.5 text-[11px] text-secondary-foreground">{{ badge }}</div>
    </div>
  </div>
</template>
