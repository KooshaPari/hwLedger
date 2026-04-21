<template>
  <div class="journey-viewer">
    <template v-if="manifest">
      <h2 class="journey-title">{{ manifest.title }}</h2>
      <div class="journey-intent">
        {{ manifest.intent || 'Journey demonstration' }}
      </div>

      <div v-if="enrichedKeyframes.length" class="journey-section">
        <KeyframeGallery
          :keyframes="enrichedKeyframes"
          :title="manifest.title"
          :journey-id="manifest.id || manifest.title || ''"
        />
      </div>

      <div class="journey-section">
        <div
          :class="['journey-status', manifest.pass ? 'pass' : 'fail']"
        >
          <div class="journey-status-badge">
            {{ manifest.pass ? 'PASS' : 'FAIL' }}
          </div>
          <span>{{ manifest.pass ? 'Journey completed successfully' : 'Journey encountered issues' }}</span>
        </div>
      </div>

      <div v-if="manifest.steps && manifest.steps.length > 0">
        <table class="journey-steps">
          <thead>
            <tr>
              <th>Step</th>
              <th>Slug</th>
              <th>Intent</th>
              <th>Screenshot</th>
              <th>Verification</th>
              <th>Score</th>
            </tr>
          </thead>
          <tbody>
            <JourneyStep
              v-for="(step, idx) in manifest.steps"
              :key="idx"
              :step="step"
              :index="idx"
            />
          </tbody>
        </table>
      </div>

      <div v-if="manifest.error" style="margin-top: 20px; padding: 12px; background-color: rgba(239, 68, 68, 0.1); border-left: 4px solid #ef4444; border-radius: 4px;">
        <div style="color: #ef4444; font-weight: 600; margin-bottom: 6px;">Error</div>
        <code style="color: #d1d5db; font-size: 12px; white-space: pre-wrap; overflow-x: auto;">{{ manifest.error }}</code>
      </div>
    </template>

    <template v-else>
      <div style="padding: 20px; text-align: center; color: var(--vp-c-text-3);">
        <p>Journey not yet recorded.</p>
        <p style="font-size: 13px; margin-top: 12px;">Run the journey recorder to capture interactions:</p>
        <code style="display: inline-block; background-color: var(--vp-c-bg-mute); padding: 8px 12px; border-radius: 4px; margin-top: 12px; font-size: 12px;">./apps/macos/HwLedgerUITests/scripts/run-journeys.sh</code>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import KeyframeGallery from './KeyframeGallery.vue'
import JourneyStep from './JourneyStep.vue'

interface Annotation {
  bbox: [number, number, number, number]
  label: string
  color?: string | null
  style?: 'solid' | 'dashed'
  note?: string | null
  kind?: 'region' | 'pointer' | 'highlight'
}

interface Manifest {
  id?: string
  title: string
  intent: string
  pass: boolean
  recording: boolean
  keyframes?: Array<{ path: string; caption: string; blind_description?: string | null; annotations?: Annotation[] | null }>
  steps?: Array<{
    index?: number
    slug: string
    intent: string
    screenshot?: string
    screenshot_path?: string
    description?: string
    blind_description?: string
    judge_score?: number
    annotations?: Annotation[] | null
  }>
  error?: string
}

const props = defineProps<{
  manifest?: string | object
}>()

const manifest = ref<Manifest | null>(null)

/**
 * Merge `steps[].annotations` onto `keyframes[i]` by position (best-effort).
 * Legacy hwLedger manifests have separate `keyframes` + `steps` arrays —
 * we pair them by caption frame-number or by array index fallback.
 */
/**
 * Synthesize keyframes from either a top-level `keyframes[]` array (legacy
 * hwLedger manifests) or from `steps[]` (canonical phenotype-journeys format
 * that only tracks `screenshot_path` per step).
 */
const enrichedKeyframes = computed(() => {
  const m = manifest.value
  if (!m) return []
  const steps = m.steps || []
  if (m.keyframes && m.keyframes.length) {
    return m.keyframes.map((kf, i) => {
      const step = steps[i]
      return {
        ...kf,
        annotations: kf.annotations ?? step?.annotations ?? null,
        blind_description: kf.blind_description ?? step?.blind_description ?? null,
      }
    })
  }
  // Derive from steps[]: path = "<dir>/<screenshot_path>" relative to the
  // manifest URL's directory (public/cli-journeys/keyframes/<id>/).
  if (!manifestUrlBase.value) return []
  const id = m.id || m.title || ''
  return steps
    .filter((s) => s.screenshot_path || s.screenshot)
    .map((s) => ({
      path: `${manifestUrlBase.value}/keyframes/${id}/${s.screenshot_path || s.screenshot}`,
      caption: s.intent,
      blind_description: s.blind_description ?? null,
      annotations: s.annotations ?? null,
    }))
})

/**
 * Base URL for resolving keyframe paths. For a manifest loaded from
 * `/cli-journeys/manifests/plan-deepseek/manifest.verified.json`, this
 * returns `/cli-journeys`.
 */
const manifestUrlBase = ref<string>('')

onMounted(async () => {
  if (!props.manifest) {
    return
  }

  if (typeof props.manifest === 'object') {
    manifest.value = props.manifest as Manifest
  } else if (typeof props.manifest === 'string') {
    // Derive base dir: `/cli-journeys/manifests/<id>/manifest.verified.json`
    // -> `/cli-journeys`.
    const url = props.manifest
    const m = url.match(/^(.*?)\/manifests\/[^/]+\/manifest(\.verified)?\.json$/)
    manifestUrlBase.value = m ? m[1] : url.replace(/\/[^/]+$/, '')
    try {
      const response = await fetch(url)
      manifest.value = await response.json()
    } catch (error) {
      console.error('Failed to load journey manifest:', error)
      manifest.value = null
    }
  }
})
</script>

<style scoped>
.journey-viewer {
  border: 1px solid var(--vp-divider);
  border-radius: 8px;
  padding: 24px;
  margin: 24px 0;
  background-color: var(--vp-c-bg-soft);
}

.journey-title {
  font-size: 24px;
  font-weight: 600;
  margin: 0 0 12px 0;
  color: var(--color-accent);
}

.journey-intent {
  font-size: 14px;
  color: var(--vp-c-text-2);
  margin-bottom: 20px;
  padding-bottom: 12px;
  border-bottom: 1px solid var(--vp-divider);
}

.journey-section {
  margin: 20px 0;
}

.journey-status {
  display: flex;
  gap: 12px;
  padding: 12px;
  border-radius: 6px;
  align-items: center;
}

.journey-status.pass {
  background-color: rgba(16, 185, 129, 0.1);
  border-left: 4px solid #10b981;
}

.journey-status.fail {
  background-color: rgba(239, 68, 68, 0.1);
  border-left: 4px solid #ef4444;
}

.journey-status-badge {
  font-weight: 600;
  font-size: 12px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.journey-status.pass .journey-status-badge {
  color: #10b981;
}

.journey-status.fail .journey-status-badge {
  color: #ef4444;
}
</style>
