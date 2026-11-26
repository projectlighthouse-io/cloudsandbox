<script setup lang="ts">
import type { Tile } from '../types/slipway'
import { clock } from './format'
import Pill from './Pill.vue'
import Sparkline from './Sparkline.vue'

defineProps<{ tile: Tile }>()

const colors = { sky: '#0ea5e9', indigo: '#6366f1', red: '#ef4444' }
</script>

<template>
  <div class="flex flex-col gap-[10px] rounded-card border border-hairline bg-white p-4 shadow-[0_1px_2px_rgba(3,7,18,0.04)]">
    <div class="flex items-center justify-between gap-2">
      <span class="text-[13px] font-medium text-muted">{{ tile.label }}</span>
      <span class="text-[12px] font-medium" :class="tile.delta.good ? 'text-[#047857]' : 'text-[#b91c1c]'">{{ tile.delta.text }}</span>
    </div>
    <div class="flex items-end justify-between gap-3">
      <span class="text-[34px] leading-none font-semibold tracking-[-0.04em] tabular-nums">{{ tile.value }}</span>
      <Sparkline :values="tile.series" :width="96" :height="34" :color="colors[tile.color]" :label="`${tile.label} over the last 7 days`" />
    </div>
    <span>
      <Pill :tone="tile.note.tone">{{ tile.note.text }}<template v-if="tile.note.at"> at {{ clock(tile.note.at) }}</template></Pill>
    </span>
  </div>
</template>
