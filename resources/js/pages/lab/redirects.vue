<script setup lang="ts">
import { router } from '@inertiajs/vue3'
import LabLayout from '../../layouts/LabLayout.vue'
</script>

<template>
  <LabLayout title="Redirects">
    <div class="card">
      <button @click="router.put('/lab/redirects')">PUT</button>
      <button @click="router.patch('/lab/redirects')">PATCH</button>
      <button @click="router.delete('/lab/redirects')">DELETE</button>
    </div>
    <p class="note">
      Each handler returns <code>Redirect::to_route("lab.redirects")</code>. After PUT, PATCH and DELETE the browser
      must follow with a GET, which a 303 guarantees; the flash message above shows which method ran.
    </p>
  </LabLayout>
</template>
