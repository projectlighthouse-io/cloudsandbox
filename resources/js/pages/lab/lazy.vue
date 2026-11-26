<script setup lang="ts">
import { Deferred, router } from '@inertiajs/vue3'
import LabLayout from '../../layouts/LabLayout.vue'

defineProps<{
  stats?: { users: number; jobs: number; computed_at: string }
  report?: { name: string; value: number }[]
  activity?: string[]
}>()
</script>

<template>
  <LabLayout title="Optional & deferred props">
    <div class="card">
      <strong>Optional</strong>
      <p class="note">Never sent on a visit. The server skips the work until a partial reload asks for <code>stats</code>.</p>
      <pre v-if="stats">{{ stats }}</pre>
      <button @click="router.reload({ only: ['stats'] })">Load stats</button>
    </div>
    <div class="card">
      <strong>Deferred, group "default"</strong> (0.8 s)
      <Deferred data="report">
        <template #fallback><p class="note">Loading report…</p></template>
        <ul><li v-for="r in report" :key="r.name">{{ r.name }}: {{ r.value }}</li></ul>
      </Deferred>
    </div>
    <div class="card">
      <strong>Deferred, group "activity"</strong> (1.5 s, its own request)
      <Deferred data="activity">
        <template #fallback><p class="note">Loading activity…</p></template>
        <ul><li v-for="a in activity" :key="a">{{ a }}</li></ul>
      </Deferred>
    </div>
    <p class="note">The page paints at once; <code>deferredProps</code> tells the client to fetch each group right after.</p>
  </LabLayout>
</template>
