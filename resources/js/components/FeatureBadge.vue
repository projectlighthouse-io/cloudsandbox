<script setup lang="ts">
import Icon from './Icon.vue'

// Names the Inertia features a page uses, next to its title.
defineProps<{ text: string }>()
</script>

<template>
  <span class="inline-flex items-center gap-[6px] rounded-full border border-accent-ring bg-accent-tint px-[10px] py-[3px] font-mono text-[11.5px] font-medium text-accent">
    <Icon name="bolt" :size="12" />{{ text }}
  </span>
</template>
