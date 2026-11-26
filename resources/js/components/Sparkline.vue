<script setup lang="ts">
import { computed } from 'vue'

// The reference's trend line: points spread across the width, 3px of headroom
// top and bottom, a 1.75px rounded stroke.
const props = defineProps<{ values: number[]; width: number; height: number; color: string; label: string }>()

const points = computed(() => {
  const v = props.values
  if (v.length < 2) return ''
  const min = Math.min(...v)
  const max = Math.max(...v)
  const step = props.width / (v.length - 1)
  return v
    .map((value, i) => {
      const y = max === min ? props.height / 2 : props.height - 3 - ((value - min) / (max - min)) * (props.height - 6)
      return `${(i * step).toFixed(1)},${y.toFixed(1)}`
    })
    .join(' ')
})
</script>

<template>
  <svg role="img" :aria-label="label" :width="width" :height="height" :viewBox="`0 0 ${width} ${height}`" class="flex-none">
    <polyline v-if="points" :points="points" fill="none" :stroke="color" stroke-width="1.75" stroke-linejoin="round" stroke-linecap="round" />
  </svg>
</template>
