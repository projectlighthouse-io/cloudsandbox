<script setup lang="ts">
import { Link } from '@inertiajs/vue3'
import LabLayout from '../../layouts/LabLayout.vue'

defineProps<{ title: string; plans?: { name: string; price: number }[]; plan_loads: number }>()
</script>

<template>
  <LabLayout :title="`Once props: ${title}`">
    <div class="card">
      <ul><li v-for="p in plans" :key="p.name">{{ p.name }}: ${{ p.price }}</li></ul>
      <div>Times the server computed <code>plans</code>: <strong>{{ plan_loads }}</strong></div>
    </div>
    <Link v-if="title === 'Pricing'" href="/lab/once/again">Go to Checkout (also shows plans)</Link>
    <Link v-else href="/lab/once">Back to Pricing</Link>
    <p class="note">
      <code>plans</code> is a once prop: the client remembers it and sends <code>X-Inertia-Except-Once-Props</code>, so
      moving between these two pages does not recompute it. The counter stays put; a full browser reload computes it again.
    </p>
  </LabLayout>
</template>
