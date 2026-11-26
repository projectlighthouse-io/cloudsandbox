<script setup lang="ts">
import { Link } from '@inertiajs/vue3'
import LabLayout from '../../layouts/LabLayout.vue'

defineProps<{ experiments: { href: string; name: string; what: string }[] }>()
</script>

<template>
  <LabLayout title="Inertia lab">
    <p class="note">Each page exercises one part of the Inertia v3 protocol as served by the <code>inertia</code> crate. Open the browser's network tab to watch the requests.</p>
    <div v-for="e in experiments" :key="e.href" class="card">
      <Link :href="e.href"><strong>{{ e.name }}</strong></Link>
      <div class="note">{{ e.what }}</div>
    </div>
    <div class="card">
      <strong>Server-side rendering</strong>
      <div class="note">Run <code>art build</code>, then <code>art ssr</code>, and start the app with <code>INERTIA_SSR_URL=http://127.0.0.1:13714</code>. View source: pages arrive rendered.</div>
    </div>
  </LabLayout>
</template>
