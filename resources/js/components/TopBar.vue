<script setup lang="ts">
import { Link, usePage } from '@inertiajs/vue3'
import { computed } from 'vue'
import type { Shared } from '../types/slipway'
import Icon from './Icon.vue'

defineEmits<{ search: [] }>()

const page = usePage<Shared>()

// The breadcrumb's last step, named after the page being shown.
const titles: Record<string, string> = {
  Overview: 'Overview',
  'servers/Index': 'Servers',
  'sites/Index': 'Sites',
  'sites/Create': 'New site',
  'sites/Edit': 'Edit site',
  'deployments/Index': 'Deployments',
  Activity: 'Activity',
  'settings/Profile': 'Settings',
  'errors/NotFound': 'Not found',
  'errors/Error': 'Error',
}
const server = computed(() => (page.props as { server?: { name: string } }).server)
const current = computed(() => server.value?.name ?? titles[page.component] ?? '')
</script>

<template>
  <header class="flex flex-wrap items-center justify-between gap-3 border-b border-hairline bg-white/90 px-8 py-3 max-[641px]:px-4 max-[641px]:py-[10px]">
    <nav aria-label="Breadcrumb" class="flex items-center gap-2 text-[14px]">
      <span class="font-medium text-subtle">{{ page.props.team.name }}</span>
      <template v-if="server">
        <span aria-hidden="true" class="text-[#9ca3af]">/</span>
        <Link href="/servers" class="font-medium text-subtle no-underline hover:text-ink">Servers</Link>
      </template>
      <span aria-hidden="true" class="text-[#9ca3af]">/</span>
      <span class="font-semibold text-ink" aria-current="page">{{ current }}</span>
    </nav>
    <div class="flex items-center gap-2">
      <button
        type="button"
        class="box-border flex h-10 min-w-[220px] cursor-pointer items-center gap-2 rounded-control border border-hairline bg-[#f9fafb] px-3 text-[14px] text-subtle"
        @click="$emit('search')"
      >
        <Icon name="search" :size="15" />
        <span class="flex-1 text-left">Search or jump to</span>
        <kbd class="rounded-[4px] border border-hairline bg-white px-[6px] py-[1px] font-mono text-[11px] text-muted">Ctrl K</kbd>
      </button>
      <button type="button" aria-label="Notifications" class="grid h-10 w-10 cursor-pointer place-items-center rounded-control border border-hairline bg-white text-muted">
        <Icon name="bell" />
      </button>
    </div>
  </header>
</template>
