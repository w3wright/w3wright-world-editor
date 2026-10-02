import { createApp } from 'vue'
import App from './App.vue'
import { router } from './router'
// Order matters: the reset first, then the utilities, then the app's own base. Each
// layer may override the one before it, and a utility that lost to the reset would be a
// bug that only shows up as one element being wrong.
import './styles/base.css'
import 'virtual:uno.css'

createApp(App).use(router).mount('#app')
