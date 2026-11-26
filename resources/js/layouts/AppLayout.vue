<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'
import Sidebar from '../components/Sidebar.vue'
import TopBar from '../components/TopBar.vue'

// The persistent shell: it stays mounted between visits, so the sidebar and
// any open overlay survive navigation.
const palette = ref(false)

const onKey = (e: KeyboardEvent) => {
  if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') {
    e.preventDefault()
    palette.value = !palette.value
  }
}
onMounted(() => document.addEventListener('keydown', onKey))
onBeforeUnmount(() => document.removeEventListener('keydown', onKey))
</script>

<template>
  <div class="flex min-h-screen flex-wrap items-stretch bg-white font-sans text-[14px] leading-[1.45] text-ink">
    <Sidebar />
    <div class="relative flex min-w-0 flex-[999_1_560px] flex-col">
      <TopBar @search="palette = true" />
      <main class="flex flex-col gap-6 px-8 pt-7 pb-10 max-[641px]:px-4 max-[641px]:pt-5 max-[641px]:pb-8">
        <slot />
      </main>
    </div>
  </div>
</template>
