<script setup lang="ts">
import { Link, usePage } from '@inertiajs/vue3'
import { computed } from 'vue'

defineProps<{ title: string }>()

const page = usePage<{ app_name: string; flash: string | null }>()
const links = [
  { href: '/lab', label: 'Lab' },
  { href: '/lab/basics', label: 'Basics' },
  { href: '/lab/lazy', label: 'Lazy' },
  { href: '/lab/merge', label: 'Merge' },
  { href: '/lab/once', label: 'Once' },
  { href: '/lab/form', label: 'Forms' },
  { href: '/lab/redirects', label: 'Redirects' },
]
const active = (href: string) => (href === '/lab' ? page.url === '/lab' : page.url.startsWith(href))
const flash = computed(() => page.props.flash)
</script>

<template>
  <div class="lab">
    <header>
      <strong>{{ page.props.app_name }}</strong>
      <nav>
        <Link v-for="l in links" :key="l.href" :href="l.href" :class="{ active: active(l.href) }">{{ l.label }}</Link>
      </nav>
    </header>
    <p v-if="flash" class="flash">{{ flash }}</p>
    <h1>{{ title }}</h1>
    <slot />
  </div>
</template>

<style>
body { margin: 0; font: 15px/1.5 system-ui, sans-serif; color: #1c1c1e; background: #fafafa; }
.lab { max-width: 760px; margin: 0 auto; padding: 16px; }
.lab header { display: flex; flex-wrap: wrap; gap: 12px; align-items: center; justify-content: space-between; padding-bottom: 12px; border-bottom: 1px solid #ddd; }
.lab nav { display: flex; flex-wrap: wrap; gap: 4px; }
.lab nav a { padding: 4px 10px; border-radius: 6px; color: #333; text-decoration: none; }
.lab nav a.active { background: #1c1c1e; color: #fff; }
.lab .flash { padding: 10px 12px; border-radius: 6px; background: #e6f4ea; color: #1e4620; }
.lab .note { color: #666; font-size: 14px; }
.lab .card { padding: 12px; margin: 12px 0; border: 1px solid #ddd; border-radius: 8px; background: #fff; }
.lab button { padding: 6px 12px; border-radius: 6px; border: 1px solid #1c1c1e; background: #fff; cursor: pointer; }
.lab button:disabled { opacity: .5; cursor: default; }
.lab input, .lab textarea { display: block; width: 100%; box-sizing: border-box; padding: 6px 8px; margin: 4px 0; border: 1px solid #bbb; border-radius: 6px; font: inherit; }
.lab .error { color: #b3261e; font-size: 14px; margin: 0 0 8px; }
.lab code { background: #eee; padding: 1px 4px; border-radius: 4px; }
</style>
