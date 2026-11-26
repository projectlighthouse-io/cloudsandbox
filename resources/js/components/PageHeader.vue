<script setup lang="ts">
import FeatureBadge from './FeatureBadge.vue'

defineProps<{ title: string; feature?: string; description?: string }>()
</script>

<template>
  <div class="flex flex-wrap items-start justify-between gap-4">
    <div class="flex min-w-0 flex-col gap-2">
      <div class="flex flex-wrap items-center gap-3">
        <h1 class="m-0 text-[30px] leading-[1.15] font-semibold tracking-[-0.035em]">{{ title }}</h1>
        <FeatureBadge v-if="feature" :text="feature" />
      </div>
      <p v-if="description" class="m-0 text-[14px] text-muted">{{ description }}</p>
    </div>
    <div v-if="$slots.default" class="flex flex-wrap gap-2"><slot /></div>
  </div>
</template>
