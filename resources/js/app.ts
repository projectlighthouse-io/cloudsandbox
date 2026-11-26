import { createInertiaApp } from '@inertiajs/vue3'
import { createApp, createSSRApp, h, type DefineComponent } from 'vue'

const pages = import.meta.glob<DefineComponent>('./pages/**/*.vue')

createInertiaApp({
  resolve: (page) => pages[`./pages/${page}.vue`](),
  setup({ el, App, props, plugin }) {
    // Hydrate markup the SSR server rendered; otherwise render from scratch.
    const create = el?.hasAttribute('data-server-rendered') ? createSSRApp : createApp
    create({ render: () => h(App, props) })
      .use(plugin)
      .mount(el)
  },
})
