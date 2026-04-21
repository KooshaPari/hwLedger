<template>
  <div class="keyframe-gallery">
    <div v-if="keyframes.length" class="keyframe-grid">
      <button
        v-for="(kf, i) in keyframes"
        :key="i"
        ref="thumbEls"
        class="keyframe-card"
        :aria-label="`Open frame ${i + 1}: ${kf.caption}`"
        @click="openAt(i)"
      >
        <div class="keyframe-thumb-wrap">
          <img :src="kf.path" :alt="kf.caption" class="keyframe-thumb" @load="onThumbLoad($event, i)" />
          <svg
            v-if="(kf.annotations?.length ?? 0) > 0 && natDims[i]"
            class="keyframe-thumb-annot"
            :viewBox="`0 0 ${natDims[i].w} ${natDims[i].h}`"
            preserveAspectRatio="xMidYMid meet"
          >
            <rect
              v-for="(a, j) in kf.annotations"
              :key="j"
              :x="a.bbox[0]"
              :y="a.bbox[1]"
              :width="a.bbox[2]"
              :height="a.bbox[3]"
              :stroke="a.color || paletteColor(j)"
              :stroke-dasharray="a.style === 'dashed' ? '6 4' : undefined"
              stroke-width="2"
              fill="none"
              opacity="0.4"
              rx="2"
            />
          </svg>
        </div>
        <div class="keyframe-card-caption">
          <span class="keyframe-num">{{ i + 1 }}.</span>
          <span class="keyframe-text">{{ kf.caption }}</span>
          <span v-if="(kf.annotations?.length ?? 0) > 0" class="keyframe-badge">
            {{ kf.annotations!.length }} annot
          </span>
        </div>
      </button>
    </div>
    <div v-else class="keyframe-empty">No keyframes.</div>

    <KeyframeLightbox
      :open="lightboxOpen"
      :frames="keyframes"
      :index="lightboxIndex"
      :journey-id="journeyId"
      @update:index="lightboxIndex = $event"
      @close="closeLightbox"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, nextTick } from 'vue'
import KeyframeLightbox from './KeyframeLightbox.vue'

interface Annotation {
  bbox: [number, number, number, number]
  label: string
  color?: string | null
  style?: 'solid' | 'dashed'
  note?: string | null
  kind?: 'region' | 'pointer' | 'highlight'
}

interface Keyframe {
  path: string
  caption: string
  annotations?: Annotation[] | null
}

const props = withDefaults(
  defineProps<{
    keyframes: Keyframe[]
    title?: string
    journeyId?: string
  }>(),
  { keyframes: () => [], journeyId: '' },
)

const PALETTE = ['#f38ba8','#a6e3a1','#f9e2af','#89b4fa','#cba6f7','#94e2d5','#fab387']
function paletteColor(i: number) { return PALETTE[i % PALETTE.length] }

const natDims = ref<Record<number, { w: number; h: number }>>({})
function onThumbLoad(ev: Event, i: number) {
  const img = ev.target as HTMLImageElement
  natDims.value[i] = { w: img.naturalWidth, h: img.naturalHeight }
}

const lightboxOpen = ref(false)
const lightboxIndex = ref(0)
const lastTrigger = ref<HTMLElement | null>(null)
const thumbEls = ref<HTMLElement[]>([])

function openAt(i: number) {
  lastTrigger.value = thumbEls.value[i] ?? null
  lightboxIndex.value = i
  lightboxOpen.value = true
}
async function closeLightbox() {
  lightboxOpen.value = false
  await nextTick()
  lastTrigger.value?.focus()
}
</script>

<style scoped>
.keyframe-gallery { margin: 20px 0; }
.keyframe-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
  gap: 14px;
}
.keyframe-card {
  all: unset;
  display: flex;
  flex-direction: column;
  gap: 8px;
  cursor: pointer;
  border: 1px solid var(--vp-divider);
  border-radius: 8px;
  overflow: hidden;
  background: var(--vp-c-bg-soft);
  transition: transform 150ms ease, box-shadow 150ms ease, border-color 150ms ease;
}
.keyframe-card:hover {
  transform: translateY(-2px);
  border-color: var(--color-accent, #89b4fa);
  box-shadow: 0 8px 24px rgba(0,0,0,0.12);
}
.keyframe-card:focus-visible {
  outline: 2px solid var(--color-accent, #89b4fa);
  outline-offset: 2px;
}
.keyframe-thumb-wrap {
  position: relative;
  aspect-ratio: 16 / 9;
  background: var(--vp-c-bg-mute);
  overflow: hidden;
}
.keyframe-thumb {
  width: 100%;
  height: 100%;
  object-fit: contain;
  display: block;
}
.keyframe-thumb-annot {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  pointer-events: none;
}
.keyframe-card-caption {
  display: flex;
  gap: 6px;
  align-items: baseline;
  padding: 10px 12px;
  font-size: 13px;
  color: var(--vp-c-text-2);
  border-top: 1px solid var(--vp-divider);
  flex-wrap: wrap;
}
.keyframe-num { font-weight: 600; color: var(--vp-c-text-1); }
.keyframe-text { flex: 1; }
.keyframe-badge {
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  font-size: 10px;
  color: #a6e3a1;
  background: rgba(166,227,161,0.12);
  padding: 2px 6px;
  border-radius: 4px;
}
.keyframe-empty {
  padding: 20px;
  text-align: center;
  color: var(--vp-c-text-3);
  border: 1px dashed var(--vp-divider);
  border-radius: 8px;
}
</style>
