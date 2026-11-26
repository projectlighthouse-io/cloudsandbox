<script setup lang="ts">
import type { Tone } from '../types/slipway'

// The design's status pill: a white capsule with a coloured dot. `live` adds
// the halo the reference gives running deployments.
withDefaults(defineProps<{ tone: Tone; live?: boolean }>(), { live: false })

const dots: Record<Tone, string> = {
  ok: 'bg-ok',
  warn: 'bg-warn',
  bad: 'bg-bad',
  info: 'bg-info',
  idle: 'bg-idle',
}
</script>

<template>
  <span class="inline-flex items-center gap-[7px] rounded-full border border-[rgba(3,7,18,0.1)] bg-white py-[2px] pr-[9px] pl-2 text-[12px] font-medium whitespace-nowrap text-[#374151]">
    <span aria-hidden="true" class="h-[6px] w-[6px] rounded-full" :class="[dots[tone], live && 'shadow-[0_0_0_3px_rgba(14,165,233,0.18)]']" />
    <slot />
  </span>
</template>
