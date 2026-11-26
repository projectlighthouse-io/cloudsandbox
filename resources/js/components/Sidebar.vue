<script setup lang="ts">
import { Link, router, usePage } from '@inertiajs/vue3'
import { ref } from 'vue'
import type { Shared } from '../types/slipway'
import Avatar from './Avatar.vue'
import Icon, { type IconName } from './Icon.vue'

const page = usePage<Shared>()
const teams = ref(false)

const nav: { href: string; label: string; icon: IconName }[] = [
  { href: '/', label: 'Overview', icon: 'overview' },
  { href: '/servers', label: 'Servers', icon: 'servers' },
  { href: '/sites', label: 'Sites', icon: 'globe' },
  { href: '/deployments', label: 'Deployments', icon: 'deploy' },
  { href: '/activity', label: 'Activity', icon: 'activity' },
  { href: '/settings/profile', label: 'Settings', icon: 'settings' },
]
const path = () => page.url.split('?')[0]
const active = (href: string) => (href === '/' ? path() === '/' : path().startsWith(href.replace('/profile', '')))

const switchTeam = (id: number) => {
  teams.value = false
  router.post(`/teams/${id}`)
}
</script>

<template>
  <aside
    class="box-border flex flex-[1_1_240px] flex-col gap-4 border-r border-hairline bg-sidebar px-3 py-4 max-[641px]:max-w-full max-[641px]:min-w-0 max-[641px]:flex-[1_1_100%] max-[641px]:gap-[10px] max-[641px]:border-r-0 max-[641px]:border-b max-[641px]:border-b-[#e5e7eb] max-[641px]:px-3 max-[641px]:py-[10px]"
  >
    <div class="flex items-center gap-[10px] px-2 py-1">
      <svg aria-hidden="true" width="26" height="26" viewBox="0 0 26 26">
        <rect width="26" height="26" rx="7" fill="#0284c7" />
        <path d="M7 18l12-9M7 13l7-5" stroke="#ffffff" stroke-width="2" stroke-linecap="round" />
      </svg>
      <span class="text-[16px] font-bold tracking-[-0.01em]">Slipway</span>
    </div>

    <div class="relative">
      <button
        type="button"
        class="flex min-h-11 w-full cursor-pointer items-center justify-between gap-2 rounded-control border border-hairline bg-white px-3 text-[14px] font-medium text-ink"
        :aria-expanded="teams"
        aria-haspopup="menu"
        @click="teams = !teams"
      >
        <span class="flex items-center gap-2">
          <span aria-hidden="true" class="grid h-5 w-5 place-items-center rounded-[5px] bg-ink text-[11px] font-bold text-white">{{ page.props.team.name[0] }}</span>
          {{ page.props.team.name }}
        </span>
        <span class="text-subtle"><Icon name="chevron-down" :size="15" /></span>
      </button>
      <!-- Not in the reference: the team menu, in the design's card style. -->
      <ul v-if="teams" role="menu" class="absolute inset-x-0 top-[calc(100%+4px)] z-20 m-0 list-none rounded-control border border-hairline bg-white p-1 shadow-[0_4px_12px_rgba(3,7,18,0.08)]">
        <li v-for="t in page.props.teams" :key="t.id" role="none">
          <button
            type="button"
            role="menuitemradio"
            :aria-checked="t.id === page.props.team.id"
            class="flex min-h-11 w-full cursor-pointer items-center justify-between rounded-[6px] border-0 bg-transparent px-3 text-left text-[14px] text-ink hover:bg-[rgba(3,7,18,0.05)]"
            @click="switchTeam(t.id)"
          >
            {{ t.name }}<span v-if="t.id === page.props.team.id" class="text-accent"><Icon name="check" :size="15" /></span>
          </button>
        </li>
      </ul>
    </div>

    <nav aria-label="Main" class="flex flex-col gap-[2px] max-[641px]:min-w-0 max-[641px]:flex-row max-[641px]:overflow-x-auto">
      <Link
        v-for="item in nav"
        :key="item.href"
        :href="item.href"
        :aria-current="active(item.href) ? 'page' : undefined"
        class="flex min-h-11 items-center gap-[10px] rounded-control px-3 text-[14px] no-underline max-[641px]:whitespace-nowrap"
        :class="active(item.href) ? 'bg-[rgba(3,7,18,0.05)] font-semibold text-ink hover:text-ink' : 'font-medium text-muted hover:text-muted'"
      >
        <Icon :name="item.icon" />{{ item.label }}
      </Link>
    </nav>

    <div class="min-h-2 flex-[1_1_0] max-[641px]:hidden" />
    <div class="flex items-center gap-[10px] border-t border-hairline p-2 max-[641px]:hidden">
      <Avatar :name="page.props.auth.user.name" />
      <div class="min-w-0">
        <div class="text-[13px] font-semibold">{{ page.props.auth.user.name }}</div>
        <div class="text-[12px] text-subtle">{{ page.props.auth.user.email }}</div>
      </div>
    </div>
  </aside>
</template>
