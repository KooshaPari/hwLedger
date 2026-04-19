import { h } from 'vue'
import type { Theme } from 'vitepress'
import DefaultTheme from 'vitepress/theme'
import './custom.css'
import JourneyViewer from './components/JourneyViewer.vue'
import KeyframeGallery from './components/KeyframeGallery.vue'
import JourneyStep from './components/JourneyStep.vue'
import JudgeScore from './components/JudgeScore.vue'

export default {
  extends: DefaultTheme,
  Layout: () => {
    return h(DefaultTheme.Layout, null, {
      // Optional: Add custom layout slots here
    })
  },
  enhanceApp({ app }) {
    app.component('JourneyViewer', JourneyViewer)
    app.component('KeyframeGallery', KeyframeGallery)
    app.component('JourneyStep', JourneyStep)
    app.component('JudgeScore', JudgeScore)
  }
} satisfies Theme
