<script setup lang="ts">
defineProps<{ title: string; hint?: string }>();
</script>

<template>
  <div class="flex flex-col items-center justify-center gap-2 py-20 text-center">
    <div class="grid size-14 place-items-center rounded-2xl bg-muted text-muted-foreground">
      <slot name="icon" />
    </div>
    <div class="font-medium">{{ title }}</div>
    <div v-if="hint" class="max-w-md text-sm text-muted-foreground">{{ hint }}</div>
    <div class="mt-2"><slot /></div>
  </div>
</template>
