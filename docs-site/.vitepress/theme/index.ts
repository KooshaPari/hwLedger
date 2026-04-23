import { h } from 'vue'
import type { Theme } from 'vitepress'
import DefaultTheme from 'vitepress/theme'
import './custom.css'
import {
  JourneyViewer,
  KeyframeGallery,
  RecordingEmbed,
  Shot,
  ShotGallery,
} from '@phenotype/journey-viewer'
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
    app.component('RecordingEmbed', RecordingEmbed)
    app.component('Shot', Shot)
    app.component('ShotGallery', ShotGallery)
  }
} satisfies Theme
