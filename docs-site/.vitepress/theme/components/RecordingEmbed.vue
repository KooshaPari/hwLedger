<template>
  <div class="recording-embed">
    <figure class="recording-figure">
      <video
        v-if="useMP4"
        class="recording-video"
        controls
        :autoplay="autoplay"
        muted
        loop
        loading="lazy"
      >
        <source :src="mp4Path" type="video/mp4" />
        <img :src="gifPath" :alt="caption || tape" loading="lazy" />
      </video>
      <img
        v-else
        class="recording-image"
        :src="gifPath"
        :alt="caption || tape"
        loading="lazy"
      />
      <figcaption v-if="caption" class="recording-caption">
        {{ caption }}
      </figcaption>
    </figure>

    <details class="keyframes-section" open>
      <summary class="keyframes-title">
        Keyframes (VLM-friendly)
      </summary>
      <div class="keyframes-grid">
        <div
          v-for="(frame, idx) in keyframesData"
          :key="idx"
          class="keyframe-item"
        >
          <img
            :src="frame.path"
            :alt="`${tape} keyframe ${idx + 1}: ${frame.alt}`"
            loading="lazy"
          />
          <p class="keyframe-caption">{{ frame.alt }}</p>
        </div>
      </div>
    </details>

    <div class="recording-links">
      <a :href="mp4Path" download class="recording-link">Download MP4</a>
      <a :href="gifPath" download class="recording-link">Download GIF</a>
      <a :href="keyframesZipPath" download class="recording-link">Keyframes ZIP</a>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'

interface KeyframeData {
  path: string
  alt: string
}

const props = withDefaults(
  defineProps<{
    tape: string
    caption?: string
    autoplay?: boolean
  }>(),
  {
    autoplay: false
  }
)

const keyframesData = ref<KeyframeData[]>([])

const gifPath = computed(() => `/cli-journeys/recordings/${props.tape}.gif`)
const mp4Path = computed(() => `/cli-journeys/recordings/${props.tape}.mp4`)
const keyframesZipPath = computed(() => `/cli-journeys/keyframes/${props.tape}.zip`)

const useMP4 = computed(() => {
  return typeof window !== 'undefined' && 'videoWidth' in document.createElement('video')
})

onMounted(async () => {
  try {
    const manifestPath = `/cli-journeys/manifests/${props.tape}/manifest.verified.json`
    const response = await fetch(manifestPath)
    if (response.ok) {
      const manifest = await response.json()
      if (manifest.steps && Array.isArray(manifest.steps)) {
        keyframesData.value = manifest.steps.map((step: any) => ({
          path: step.screenshot_path || `/cli-journeys/keyframes/${props.tape}/frame-${String(step.index + 1).padStart(3, '0')}.png`,
          alt: step.intent || `Step ${step.index + 1}`
        }))
      }
    }
  } catch (error) {
    console.warn(`Could not load manifest for ${props.tape}:`, error)
  }
})
</script>

<style scoped>
.recording-embed {
  margin: 24px 0;
  padding: 16px;
  border: 1px solid var(--vp-divider);
  border-radius: 8px;
  background-color: var(--vp-c-bg-soft);
}

.recording-figure {
  margin: 0 0 20px 0;
  padding: 0;
  text-align: center;
}

.recording-video,
.recording-image {
  max-width: 100%;
  height: auto;
  border-radius: 6px;
  display: block;
  margin: 0 auto;
  background-color: var(--vp-c-bg-mute);
}

.recording-caption {
  margin-top: 8px;
  font-size: 13px;
  color: var(--vp-c-text-2);
  font-style: italic;
}

.keyframes-section {
  margin: 20px 0;
  padding: 12px;
  border: 1px solid var(--vp-divider);
  border-radius: 6px;
  background-color: var(--vp-c-bg-mute);
}

.keyframes-title {
  cursor: pointer;
  font-weight: 600;
  color: var(--vp-c-text-1);
  padding: 4px 0;
  user-select: none;
}

.keyframes-title:hover {
  color: var(--color-accent);
}

.keyframes-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
  gap: 12px;
  margin-top: 12px;
}

.keyframe-item {
  border: 1px solid var(--vp-divider);
  border-radius: 4px;
  overflow: hidden;
  background-color: var(--vp-c-bg);
}

.keyframe-item img {
  display: block;
  width: 100%;
  height: auto;
  aspect-ratio: 16 / 9;
  object-fit: cover;
}

.keyframe-caption {
  padding: 8px;
  margin: 0;
  font-size: 11px;
  color: var(--vp-c-text-2);
  line-height: 1.3;
  overflow: hidden;
  text-overflow: ellipsis;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
}

.recording-links {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
  margin-top: 16px;
}

.recording-link {
  display: inline-block;
  padding: 6px 12px;
  font-size: 13px;
  border: 1px solid var(--vp-divider);
  border-radius: 4px;
  background-color: var(--vp-c-bg-mute);
  color: var(--color-accent);
  text-decoration: none;
  transition: all 0.3s ease;
}

.recording-link:hover {
  background-color: var(--color-accent);
  color: white;
  border-color: var(--color-accent);
}

@media (prefers-color-scheme: dark) {
  .recording-embed {
    background-color: rgba(255, 255, 255, 0.05);
  }

  .keyframes-section {
    background-color: rgba(255, 255, 255, 0.02);
  }

  .keyframe-item {
    background-color: rgba(255, 255, 255, 0.03);
  }
}
</style>
