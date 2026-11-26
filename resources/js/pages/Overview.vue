<script setup lang="ts">
import { Deferred, Link, usePage, usePoll } from '@inertiajs/vue3'
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import Button from '../components/Button.vue'
import Card from '../components/Card.vue'
import { count, firstName, when } from '../components/format'
import Icon from '../components/Icon.vue'
import PageHeader from '../components/PageHeader.vue'
import Skeleton from '../components/Skeleton.vue'
import Sparkline from '../components/Sparkline.vue'
import StatTile from '../components/StatTile.vue'
import StatusPill from '../components/StatusPill.vue'
import AppLayout from '../layouts/AppLayout.vue'
import type { Deployment, Shared, Tile, Usage } from '../types/slipway'

defineOptions({ layout: AppLayout })

const props = defineProps<{
  summary: string
  tiles: Tile[]
  recent: Deployment[]
  /** Deferred: the first servers' usage. */
  usage?: Usage[]
  /** Deferred in a slower group: the rest. */
  more_usage?: Usage[]
}>()

const page = usePage<Shared>()

// Polling: every 5 seconds, reload only the usage props.
const updatedAt = ref(Date.now())
usePoll(5000, { only: ['usage', 'more_usage'], onSuccess: () => (updatedAt.value = Date.now()) })

const now = ref(Date.now())
let tick: ReturnType<typeof setInterval>
onMounted(() => (tick = setInterval(() => (now.value = Date.now()), 1000)))
onBeforeUnmount(() => clearInterval(tick))
const updated = computed(() => `${Math.max(0, Math.round((now.value - updatedAt.value) / 1000))}s ago`)

const serverCount = computed(() => props.tiles[0]?.value ?? 0)
const waiting = computed(() => serverCount.value - (props.usage?.length ?? 0))
const hot = (cpu: number | null) => (cpu ?? 0) >= 80
</script>

<template>
  <PageHeader title="Overview" feature="Deferred props · Polling" :description="`${page.props.team.name} · ${summary}`">
    <Button variant="primary" icon="plus" href="/servers">Add server</Button>
  </PageHeader>

  <div class="grid grid-cols-[repeat(auto-fit,minmax(200px,1fr))] gap-4">
    <StatTile v-for="tile in tiles" :key="tile.label" :tile="tile" />
  </div>

  <div class="flex flex-wrap items-start gap-4">
    <div class="min-w-0 flex-[3_1_420px]">
      <Card title="Recent deployments">
        <template #actions><Link href="/deployments" class="text-[13px] font-medium underline underline-offset-2">View all</Link></template>
        <div>
          <Link
            v-for="d in recent"
            :key="d.id"
            href="/deployments"
            class="flex flex-wrap items-center gap-x-3 gap-y-2 border-b border-[#f3f4f6] px-4 py-3 text-ink no-underline hover:text-ink"
          >
            <span class="min-w-0 flex-[1_1_220px]">
              <span class="block font-semibold">{{ d.site }}</span>
              <span class="block truncate text-[13px] text-muted">{{ d.message }} · {{ firstName(d.author) }}</span>
            </span>
            <span class="font-mono text-[12.5px] text-muted">{{ d.commit }}</span>
            <StatusPill :status="d.status" />
            <span class="w-[74px] text-right text-[12px] text-subtle">{{ when(d.startedAt, now) }}</span>
          </Link>
          <p v-if="!recent.length" class="m-0 px-4 py-6 text-center text-[13px] text-muted">No deployments yet.</p>
        </div>
      </Card>
    </div>

    <div class="min-w-0 flex-[2_1_380px]">
      <Card title="Resource usage">
        <template #actions>
          <span class="inline-flex items-center gap-[6px] text-[12px] text-muted"><Icon name="clock" :size="13" />Polling every 5s · updated {{ updated }}</span>
        </template>
        <div>
          <Deferred data="usage">
            <template #fallback>
              <div v-for="i in 4" :key="i" class="flex items-center gap-4 border-b border-[#f3f4f6] p-4">
                <Skeleton width="110px" height="12px" /><span class="flex-1" /><Skeleton width="120px" height="24px" /><Skeleton width="48px" height="12px" /><Skeleton width="48px" height="12px" />
              </div>
            </template>
            <div v-for="u in usage" :key="u.serverId" class="flex flex-wrap items-center gap-x-4 gap-y-2 border-b border-[#f3f4f6] px-4 py-3">
              <span class="flex-[1_1_110px] font-mono text-[12.5px] font-medium">{{ u.name }}</span>
              <Sparkline :values="u.trend" :width="120" :height="32" :color="hot(u.cpu) ? '#b45309' : '#0284c7'" :label="`CPU trend for ${u.name}`" />
              <span class="w-16 text-right font-semibold tabular-nums" :class="hot(u.cpu) ? 'text-[#b45309]' : 'text-ink'">{{ u.cpu ?? 'n/a' }}<template v-if="u.cpu !== null">%</template> <span class="text-[12px] font-normal text-subtle">cpu</span></span>
              <span class="w-16 text-right tabular-nums">{{ u.memory ?? 'n/a' }}<template v-if="u.memory !== null">%</template> <span class="text-[12px] text-subtle">mem</span></span>
            </div>
          </Deferred>
          <Deferred data="more_usage">
            <template #fallback>
              <div v-for="i in 2" :key="i" class="flex items-center gap-4 border-b border-[#f3f4f6] p-4">
                <Skeleton width="110px" height="12px" /><span class="flex-1" /><Skeleton width="120px" height="24px" /><Skeleton width="48px" height="12px" /><Skeleton width="48px" height="12px" />
              </div>
              <div class="px-4 py-[10px] text-[12px] text-muted">{{ count(waiting) }} servers are still loading. Deferred props arrive after first paint.</div>
            </template>
            <div v-for="u in more_usage" :key="u.serverId" class="flex flex-wrap items-center gap-x-4 gap-y-2 border-b border-[#f3f4f6] px-4 py-3 last:border-b-0">
              <span class="flex-[1_1_110px] font-mono text-[12.5px] font-medium">{{ u.name }}</span>
              <Sparkline :values="u.trend" :width="120" :height="32" :color="hot(u.cpu) ? '#b45309' : '#0284c7'" :label="`CPU trend for ${u.name}`" />
              <span class="w-16 text-right font-semibold tabular-nums" :class="hot(u.cpu) ? 'text-[#b45309]' : 'text-ink'">{{ u.cpu ?? 'n/a' }}<template v-if="u.cpu !== null">%</template> <span class="text-[12px] font-normal text-subtle">cpu</span></span>
              <span class="w-16 text-right tabular-nums">{{ u.memory ?? 'n/a' }}<template v-if="u.memory !== null">%</template> <span class="text-[12px] text-subtle">mem</span></span>
            </div>
          </Deferred>
        </div>
      </Card>
    </div>
  </div>
</template>
