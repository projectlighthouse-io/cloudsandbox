<script setup lang="ts">
import { router } from '@inertiajs/vue3'
import LabLayout from '../../layouts/LabLayout.vue'

const props = defineProps<{ page: number; has_more: boolean; items: { id: number; text: string }[] }>()

const more = () => router.reload({ only: ['items', 'page', 'has_more'], data: { page: props.page + 1 } })
const reset = () => router.reload({ only: ['items', 'page', 'has_more'], data: { page: 1 }, reset: ['items'] })
</script>

<template>
  <LabLayout title="Merge props">
    <ol class="card">
      <li v-for="item in items" :key="item.id">{{ item.text }}</li>
    </ol>
    <button :disabled="!has_more" @click="more">Load more (page {{ page + 1 }})</button>
    <button @click="reset">Reset</button>
    <p class="note">
      Each page returns 5 items; the server marks <code>items</code> in <code>mergeProps</code> (matched on <code>id</code>),
      so the client appends instead of replacing. Reset sends <code>X-Inertia-Reset</code> and the list starts over.
    </p>
  </LabLayout>
</template>
