import { defineAsyncComponent, h } from 'vue'
import type { Theme } from 'vitepress'
import DefaultTheme from 'vitepress/theme'
import './custom.css'
import JourneyStep from './components/JourneyStep.vue'
import JudgeScore from './components/JudgeScore.vue'

const JourneyViewer = defineAsyncComponent(() => import('@phenotype/journey-viewer/JourneyViewer.vue'))
const KeyframeGallery = defineAsyncComponent(() => import('@phenotype/journey-viewer/KeyframeGallery.vue'))
const RecordingEmbed = defineAsyncComponent(() => import('@phenotype/journey-viewer/RecordingEmbed.vue'))
const Shot = defineAsyncComponent(() => import('@phenotype/journey-viewer/Shot.vue'))
const ShotGallery = defineAsyncComponent(() => import('@phenotype/journey-viewer/ShotGallery.vue'))

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
    app.component('RecordingEmbed', RecordingEmbed)
    app.component('Shot', Shot)
    app.component('ShotGallery', ShotGallery)
  }
} satisfies Theme
