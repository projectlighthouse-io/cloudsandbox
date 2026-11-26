<script setup lang="ts">
import type { Home } from '../types/Home'

defineProps<Home>()
</script>

<template>
  <main>
    <h1>{{ app_name }}</h1>
    <p>It works.</p>
  </main>
</template>
