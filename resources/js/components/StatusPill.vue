<script setup lang="ts">
import { computed } from 'vue'
import type { Tone } from '../types/slipway'
import Pill from './Pill.vue'

// Status words, as the design writes them, and their dot colour.
const props = defineProps<{ status: string }>()

const known: Record<string, [string, Tone]> = {
  running: ['Running', 'info'],
  queued: ['Queued', 'warn'],
  finished: ['Finished', 'ok'],
  failed: ['Failed', 'bad'],
  cancelled: ['Cancelled', 'idle'],
  healthy: ['Healthy', 'ok'],
  degraded: ['High load', 'warn'],
  provisioning: ['Provisioning', 'info'],
  offline: ['Offline', 'bad'],
  deployed: ['Deployed', 'ok'],
  deploying: ['Deploying', 'info'],
  suspended: ['Suspended', 'idle'],
}
const shown = computed(() => known[props.status] ?? [props.status, 'idle' as Tone])
</script>

<template>
  <Pill :tone="shown[1]" :live="status === 'running'">{{ shown[0] }}</Pill>
</template>
