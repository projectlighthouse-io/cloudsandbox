import { createInertiaApp } from '@inertiajs/vue3'
import createServer from '@inertiajs/vue3/server'
import { createSSRApp, h, type DefineComponent } from 'vue'
import { renderToString } from 'vue/server-renderer'

const pages = import.meta.glob<DefineComponent>('./pages/**/*.vue')

// The app posts each first visit's page to http://127.0.0.1:13714/render
// when INERTIA_SSR_URL is set. Bound to localhost: not for the outside world.
createServer(
  (page) =>
    createInertiaApp({
      page,
      render: renderToString,
      resolve: (component) => pages[`./pages/${component}.vue`](),
      setup({ App, props, plugin }) {
        return createSSRApp({ render: () => h(App, props) }).use(plugin)
      },
    }),
  { host: '127.0.0.1' },
)
