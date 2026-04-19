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

    <details class="keyframes-section" :open="keyframesData.length > 0">
      <summary class="keyframes-title">
        Keyframes ({{ keyframesData.length }} — VLM-friendly)
      </summary>
      <div v-if="keyframesData.length > 0" class="keyframes-grid">
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
          <p class="keyframe-caption">{{ idx + 1 }}. {{ frame.alt }}</p>
        </div>
      </div>
      <p v-else-if="manifestMissing" class="keyframes-empty">
        No verified manifest yet — this journey ran but hasn't been
        blackbox-verified. The raw keyframes are in
        <a :href="keyframesRepoUrl" target="_blank" rel="noopener">the repo</a>.
      </p>
      <p v-else class="keyframes-empty">Loading manifest…</p>
    </details>

    <div class="recording-links">
      <a :href="mp4Path" download class="recording-link">Download MP4</a>
      <a :href="gifPath" download class="recording-link">Download GIF</a>
      <a :href="keyframesRepoUrl" target="_blank" rel="noopener" class="recording-link">
        Keyframes (repo)
      </a>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { withBase, useData } from 'vitepress'

interface KeyframeData {
  path: string
  alt: string
}

const props = withDefaults(
  defineProps<{
    tape: string
    caption?: string
    autoplay?: boolean
    // Max keyframes to render in the gallery. Default 12 keeps the VLM
    // context budget under ~30k tokens per journey.
    maxKeyframes?: number
  }>(),
  {
    autoplay: false,
    maxKeyframes: 12
  }
)

const { site } = useData()
const keyframesData = ref<KeyframeData[]>([])
const manifestMissing = ref(false)

// base-aware URL helpers. VitePress's withBase() handles the leading
// `/hwLedger/` prefix in production so assets resolve correctly.
const gifPath = computed(() => withBase(`/cli-journeys/recordings/${props.tape}.gif`))
const mp4Path = computed(() => withBase(`/cli-journeys/recordings/${props.tape}.mp4`))
// No pre-built ZIPs exist — link to the keyframes directory listing instead.
// Users who want the raw keyframes can curl individual frames or pull from
// apps/cli-journeys/keyframes/<tape>/ in the repo.
const keyframesRepoUrl = computed(
  () => `https://github.com/KooshaPari/hwLedger/tree/main/apps/cli-journeys/keyframes/${props.tape}`
)

const useMP4 = computed(() => {
  return typeof window !== 'undefined' && 'videoWidth' in document.createElement('video')
})

function normaliseFramePath(raw: string | undefined, index: number): string {
  if (!raw) {
    return withBase(
      `/cli-journeys/keyframes/${props.tape}/frame-${String(index + 1).padStart(3, '0')}.png`
    )
  }
  // Manifests sometimes store paths like "step-000-…png" (WP25 UI harness
  // format) or "frame-001.png" (WP26 CLI format); both are under the tape
  // directory. If the path is absolute (starts with /) or already fully
  // qualified, pass through withBase unchanged.
  if (raw.startsWith('http')) {
    return raw
  }
  if (raw.startsWith('/')) {
    return withBase(raw)
  }
  return withBase(`/cli-journeys/keyframes/${props.tape}/${raw}`)
}

// VitePress dev server returns index.html (200 OK) for missing paths, so
// `response.ok` is not sufficient — we must also reject HTML responses.
function isJsonResponse(response: Response): boolean {
  const ct = response.headers.get('content-type') || ''
  return ct.includes('application/json') || ct.includes('json')
}

onMounted(async () => {
  try {
    const tryFetch = async (path: string): Promise<Response | null> => {
      const res = await fetch(withBase(path))
      if (!res.ok) return null
      if (!isJsonResponse(res)) return null
      return res
    }

    let response = await tryFetch(
      `/cli-journeys/manifests/${props.tape}/manifest.verified.json`
    )
    if (!response) {
      response = await tryFetch(
        `/cli-journeys/manifests/${props.tape}/manifest.json`
      )
    }
    if (response) {
      const manifest = await response.json()
      if (manifest.steps && Array.isArray(manifest.steps)) {
        keyframesData.value = manifest.steps
          .slice(0, props.maxKeyframes)
          .map((step: any, index: number) => ({
            path: normaliseFramePath(step.screenshot_path, step.index ?? index),
            alt: step.intent || step.slug || `Step ${step.index ?? index + 1}`
          }))
      }
    } else {
      manifestMissing.value = true
    }
  } catch (error) {
    manifestMissing.value = true
    // eslint-disable-next-line no-console
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
