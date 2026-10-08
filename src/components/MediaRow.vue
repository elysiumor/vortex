<script setup lang="ts">
import { ref } from "vue";
import { ChevronLeft, ChevronRight } from "@lucide/vue";
import { Carousel, CarouselContent, CarouselNext, CarouselPrevious, type CarouselApi } from "@/components/ui/carousel";

/**
 * A titled row that scrolls sideways, as on a streaming home page: drag it,
 * use the edge arrows, or move through it with the keyboard. Each card goes
 * in a `CarouselItem` in the default slot.
 */
defineProps<{ title: string; count?: number; seeAll?: boolean }>();
const emit = defineEmits<{ seeAll: [] }>();

const api = ref<CarouselApi>();

// The carousel moves its track with a transform, so a card focused from the
// keyboard is not scrolled into view by the browser; bring it in ourselves.
function onFocusIn(e: FocusEvent) {
  const a = api.value;
  const slide = (e.target as HTMLElement).closest<HTMLElement>('[data-slot="carousel-item"]');
  if (!a || !slide) return;
  // Tab focus scrolls the clipped viewport natively; undo that, the carousel
  // positions the track itself.
  a.rootNode().scrollLeft = 0;
  const index = a.slideNodes().indexOf(slide);
  if (index < 0) return;
  const card = slide.getBoundingClientRect();
  const view = a.rootNode().getBoundingClientRect();
  if (card.left >= view.left && card.right <= view.right) return;
  // scrollTo takes a snap (a page of slides with slidesToScroll "auto"), not a slide.
  const snap = a.internalEngine().slideRegistry.findIndex((group) => group.includes(index));
  a.scrollTo(snap >= 0 ? snap : index);
}
</script>

<template>
  <section class="group/row">
    <div class="mb-1 flex items-end gap-3 px-10">
      <h2 class="section-title text-[1.3rem]">{{ title }}</h2>
      <span v-if="count" class="pb-0.5 text-sm text-muted-foreground">{{ count }}</span>
      <button v-if="seeAll" class="ml-auto flex items-center gap-0.5 pb-0.5 text-sm font-semibold text-primary transition-[gap] hover:gap-1.5" @click="emit('seeAll')">
        See all <ChevronRight class="size-4" />
      </button>
    </div>
    <Carousel :opts="{ align: 'start', dragFree: true, slidesToScroll: 'auto', containScroll: 'trimSnaps' }" @init-api="(a) => (api = a)" @focusin="onFocusIn">
      <CarouselContent class="ml-0 pb-4 pt-4">
        <slot />
      </CarouselContent>
      <CarouselPrevious variant="ghost" class="glass left-4 top-[42%] z-20 size-11 border border-border shadow-xl opacity-0 transition-opacity group-hover/row:opacity-100 disabled:!opacity-0">
        <ChevronLeft class="size-5" />
      </CarouselPrevious>
      <CarouselNext variant="ghost" class="glass right-4 top-[42%] z-20 size-11 border border-border shadow-xl opacity-0 transition-opacity group-hover/row:opacity-100 disabled:!opacity-0">
        <ChevronRight class="size-5" />
      </CarouselNext>
    </Carousel>
  </section>
</template>
