import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import { registerBuiltinCommands } from './commands/builtin'
import { i18n } from './i18n'
import './styles/fonts.css'
import './styles/tokens.css'

const app = createApp(App).use(createPinia()).use(i18n)
registerBuiltinCommands()
app.mount('#app')
