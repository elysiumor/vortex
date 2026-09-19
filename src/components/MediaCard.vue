<script setup lang="ts">
import { computed, ref, onMounted, onUnmounted } from "vue";
import { Play, Check, X } from "@lucide/vue";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";

const props = defineProps<{
  title: string;
  poster: string | null;
  subtitle?: string;
  meta?: string;
  /** 0..1 progress bar under the poster */
  progress?: number | null;
  done?: boolean;
  badge?: string;
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
    class="group relative flex flex-col rounded-xl bg-card border border-border/60 overflow-hidden cursor-pointer
           transition-all duration-200 hover:-translate-y-0.5 hover:shadow-xl hover:shadow-black/20 hover:border-border
           focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
    :class="{ 'opacity-60': unavailable }"
    @click="emit('open')"
    @dblclick="playable && emit('play')"
  >
    <div class="relative aspect-[2/3] bg-muted overflow-hidden">
      <img v-if="poster" :src="poster" :alt="title" loading="lazy"
           class="h-full w-full object-cover transition-transform duration-300 group-hover:scale-[1.03]" />
      <div v-else class="flex h-full w-full items-center justify-center text-4xl font-semibold text-muted-foreground/60 bg-gradient-to-br from-muted to-accent">
        {{ initial }}
      </div>

      <div v-if="done" class="absolute top-2 left-2 grid size-6 place-items-center rounded-full bg-success text-white shadow">
        <Check class="size-3.5" />
      </div>
      <Button v-if="dismissable" variant="secondary" size="icon-xs"
              class="absolute top-2 right-2 rounded-full opacity-0 group-hover:opacity-100 group-focus-within:opacity-100 bg-black/60 text-white hover:bg-destructive border-0"
              title="Remove from Continue watching (keeps progress)" @click.stop="emit('dismiss')">
        <X />
      </Button>

      <div v-if="playable" class="absolute inset-0 flex items-end justify-center pb-3 opacity-0 group-hover:opacity-100 group-focus-visible:opacity-100 transition-opacity bg-gradient-to-t from-black/70 via-black/10 to-transparent">
        <Button size="sm" class="shadow-lg" :disabled="unavailable" @click.stop="emit('play')">
          <Play class="fill-current" /> {{ unavailable ? "Offline" : "Play" }}
        </Button>
      </div>

      <div v-if="progress != null && progress > 0" class="absolute inset-x-0 bottom-0 h-1 bg-black/40">
        <div class="h-full bg-primary" :style="{ width: Math.min(100, progress * 100) + '%' }"></div>
      </div>
    </div>

    <div class="flex flex-col gap-1 p-3">
      <div class="font-medium leading-tight line-clamp-2">{{ title }}</div>
      <div v-if="subtitle" class="text-xs text-muted-foreground line-clamp-1">{{ subtitle }}</div>
      <div v-if="meta" class="text-xs text-primary line-clamp-1">{{ meta }}</div>
      <Badge v-if="badge" variant="secondary" class="mt-1 w-fit font-normal text-[11px]">{{ badge }}</Badge>
    </div>
  </div>
</template>
