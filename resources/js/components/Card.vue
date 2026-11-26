<script setup lang="ts">
// The design's card. With a `title`, it gets the bordered header row.
defineProps<{ title?: string }>()
</script>

<template>
  <section class="min-w-0 rounded-card border border-hairline bg-white shadow-[0_1px_2px_rgba(3,7,18,0.04)]">
    <div v-if="title" class="flex flex-wrap items-center justify-between gap-2 border-b border-hairline px-4 py-[14px]">
      <h2 class="m-0 text-[14px] font-semibold">{{ title }}</h2>
      <slot name="actions" />
    </div>
    <slot />
  </section>
</template>
