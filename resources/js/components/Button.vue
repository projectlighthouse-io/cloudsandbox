<script setup lang="ts">
import { Link } from '@inertiajs/vue3'
import { computed } from 'vue'
import Icon, { type IconName } from './Icon.vue'

// The design's buttons. A `href` renders an Inertia link with the same look.
const props = withDefaults(
  defineProps<{ variant?: 'primary' | 'secondary' | 'danger'; small?: boolean; icon?: IconName; href?: string; type?: 'button' | 'submit' }>(),
  { variant: 'secondary', small: false, type: 'button' },
)

const variants = {
  primary: 'bg-ink text-white border-ink shadow-[0_1px_2px_rgba(3,7,18,0.12)] hover:bg-[#1f2937] hover:text-white',
  secondary: 'bg-white text-ink border-hairline shadow-[0_1px_2px_rgba(3,7,18,0.04)] hover:bg-[#f9fafb] hover:text-ink',
  danger: 'bg-white text-[#b91c1c] border-[rgba(239,68,68,0.3)] hover:bg-[#fef2f2] hover:text-[#b91c1c]',
}
const classes = computed(() => [
  'inline-flex cursor-pointer items-center gap-2 rounded-control border font-medium whitespace-nowrap no-underline disabled:cursor-not-allowed disabled:opacity-50',
  props.small ? 'h-8 px-[10px] text-[13px]' : 'h-10 px-[14px] text-[14px]',
  variants[props.variant],
])
</script>

<template>
  <Link v-if="href" :href="href" :class="classes"><Icon v-if="icon" :name="icon" :size="small ? 14 : 16" /><slot /></Link>
  <button v-else :type="type" :class="classes"><Icon v-if="icon" :name="icon" :size="small ? 14 : 16" /><slot /></button>
</template>
