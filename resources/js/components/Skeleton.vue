<script setup lang="ts">
// A grey placeholder block, as the reference draws loading rows.
defineProps<{ width: string; height: string }>()
</script>

<template>
  <span aria-hidden="true" class="block rounded-[4px] bg-[#e5e7eb]" :style="{ width, height }" />
</template>
