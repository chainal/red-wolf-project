import { createApp } from 'vue'

import App from './App.vue'
import Varlet from '@varlet/ui'

import 'leaflet/dist/leaflet.css'
import '@varlet/ui/es/style'
import './my-style.css'

createApp(App).use(Varlet).mount('#app')
