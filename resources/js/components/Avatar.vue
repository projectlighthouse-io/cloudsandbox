<script setup lang="ts">
import { computed } from 'vue'

const props = withDefaults(defineProps<{ name: string; size?: number }>(), { size: 32 })

// Each person keeps one colour across the app.
const colors: Record<string, string> = {
  'Elizabeth Lane': '#8b5cf6',
  'Tomas Vidal': '#0ea5e9',
  'Imani Okoro': '#f59e0b',
}
const initials = computed(() => props.name.split(' ').map((p) => p[0]).slice(0, 2).join('').toUpperCase())
const color = computed(() => colors[props.name] ?? '#6b7280')
</script>

<template>
  <span
    aria-hidden="true"
    class="grid flex-none place-items-center rounded-full font-bold text-white"
    :style="{ width: `${size}px`, height: `${size}px`, background: color, fontSize: `${Math.round(size * 0.375)}px` }"
  >{{ initials }}</span>
</template>
