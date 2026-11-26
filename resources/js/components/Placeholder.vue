<script setup lang="ts">
import { usePage } from '@inertiajs/vue3'
import Card from './Card.vue'
import PageHeader from './PageHeader.vue'

// Stands in for a page not yet built from the design.
const page = usePage()
</script>

<template>
  <PageHeader :title="page.component" description="This page is next to be built from the design. Its props are below." />
  <Card>
    <details class="p-4">
      <summary class="cursor-pointer text-[13px] font-medium text-muted">Props</summary>
      <pre class="mt-3 overflow-x-auto font-mono text-[12px]">{{ page.props }}</pre>
    </details>
  </Card>
</template>
