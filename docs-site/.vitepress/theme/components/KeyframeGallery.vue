<template>
  <div class="keyframe-gallery">
    <div v-if="keyframes.length" class="keyframe-grid">
      <div
        v-for="(kf, i) in keyframes"
        :key="i"
        class="keyframe-card"
      >
        <button
          ref="thumbEls"
          class="keyframe-thumb-btn"
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
            <span class="keyframe-expand-pill" aria-hidden="true">⤢ Click to expand</span>
          </div>
        </button>
        <div
          class="keyframe-card-caption"
          :class="{ 'is-expanded': expanded[i] }"
          tabindex="0"
          :aria-label="`Caption for frame ${i + 1}. Use arrow keys to scroll.`"
        >
          <div class="keyframe-caption-body">
            <div class="keyframe-caption-row">
              <span class="keyframe-num">{{ i + 1 }}.</span>
              <span class="keyframe-label keyframe-label-intent">Intent</span>
              <span class="keyframe-text">{{ kf.caption || '—' }}</span>
              <span v-if="(kf.annotations?.length ?? 0) > 0" class="keyframe-badge">
                {{ kf.annotations!.length }} annot
              </span>
            </div>
            <div class="keyframe-caption-row keyframe-caption-row-blind">
              <span class="keyframe-num keyframe-num-blank" aria-hidden="true">&nbsp;</span>
              <span class="keyframe-label keyframe-label-blind">Blind</span>
              <span class="keyframe-text keyframe-text-blind">{{ kf.blind_description || '—' }}</span>
            </div>
          </div>
        </div>
        <button
          v-if="isOverflowing(i, kf.caption)"
          type="button"
          class="keyframe-toggle"
          :aria-expanded="!!expanded[i]"
          @click="toggleExpanded(i)"
        >
          {{ expanded[i] ? 'Show less ▴' : 'Show more ▾' }}
        </button>
      </div>
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
  blind_description?: string | null
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
const expanded = ref<Record<number, boolean>>({})

// Heuristic: consider caption overflowing (i.e. would be clamped) when it is
// longer than ~140 chars, or contains more than 2 "labels" (Intent: / Blind:),
// or has an explicit newline. This avoids having to measure DOM boxes.
function isOverflowing(_i: number, caption: string): boolean {
  if (!caption) return false
  if (caption.length > 140) return true
  if (caption.includes('\n')) return true
  const labelMatches = caption.match(/\b(Intent|Blind|Observation|Expected|Actual):/g)
  return !!labelMatches && labelMatches.length >= 2
}

function toggleExpanded(i: number) {
  expanded.value = { ...expanded.value, [i]: !expanded.value[i] }
}

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
  display: flex;
  flex-direction: column;
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
.keyframe-thumb-btn {
  all: unset;
  display: block;
  cursor: zoom-in;
  width: 100%;
}
.keyframe-thumb-btn:focus-visible {
  outline: 2px solid var(--color-accent, #89b4fa);
  outline-offset: -2px;
}
.keyframe-thumb-wrap {
  position: relative;
  aspect-ratio: 16 / 9;
  background: var(--vp-c-bg-mute);
  overflow: hidden;
  cursor: zoom-in;
}
.keyframe-thumb {
  width: 100%;
  height: 100%;
  object-fit: contain;
  display: block;
  transition: filter 120ms ease;
}
.keyframe-thumb-btn:hover .keyframe-thumb,
.keyframe-thumb-btn:focus-visible .keyframe-thumb {
  filter: brightness(0.55);
}
.keyframe-expand-pill {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%) scale(0.96);
  padding: 6px 12px;
  border-radius: 999px;
  background: rgba(17, 17, 27, 0.88);
  color: #cdd6f4;
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  font-size: 12px;
  font-weight: 500;
  letter-spacing: 0.02em;
  white-space: nowrap;
  box-shadow: 0 6px 18px rgba(0, 0, 0, 0.5);
  opacity: 0;
  pointer-events: none;
  transition: opacity 120ms ease, transform 120ms ease;
}
.keyframe-thumb-btn:hover .keyframe-expand-pill,
.keyframe-thumb-btn:focus-visible .keyframe-expand-pill {
  opacity: 1;
  transform: translate(-50%, -50%) scale(1);
}
.keyframe-thumb-annot {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  pointer-events: none;
}
.keyframe-card-caption {
  /* Collapsed: clamp to 3 lines. Expanded: max-height w/ scroll. */
  padding: 10px 12px;
  font-size: 13px;
  color: var(--vp-c-text-2);
  border-top: 1px solid var(--vp-divider);
  max-height: 4.5rem; /* ~3 lines at 13px * 1.5 line-height */
  overflow: hidden;
  transition: max-height 150ms ease;
}
.keyframe-card-caption:focus-visible {
  outline: 2px solid var(--color-accent, #89b4fa);
  outline-offset: -2px;
}
.keyframe-card-caption .keyframe-caption-body {
  display: -webkit-box;
  -webkit-line-clamp: 3;
  -webkit-box-orient: vertical;
  overflow: hidden;
  line-height: 1.5;
}
.keyframe-card-caption.is-expanded {
  max-height: 16rem;
  overflow-y: auto;
}
.keyframe-card-caption.is-expanded .keyframe-caption-body {
  display: block;
  -webkit-line-clamp: unset;
  overflow: visible;
}
.keyframe-caption-body { display: block; word-break: break-word; }
.keyframe-caption-row {
  display: flex;
  gap: 6px;
  align-items: baseline;
  min-width: 0;
}
.keyframe-caption-row + .keyframe-caption-row { margin-top: 4px; }
.keyframe-caption-row-blind { opacity: 0.9; }
.keyframe-num { font-weight: 600; color: var(--vp-c-text-1); margin-right: 4px; flex-shrink: 0; }
.keyframe-num-blank { visibility: hidden; }
.keyframe-label {
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  font-size: 10px;
  font-weight: 600;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  padding: 1px 6px;
  border-radius: 4px;
  flex-shrink: 0;
}
.keyframe-label-intent {
  color: var(--color-accent, #89b4fa);
  background: rgba(137, 180, 250, 0.12);
}
.keyframe-label-blind {
  color: var(--vp-c-text-3);
  background: rgba(205, 214, 244, 0.06);
  border: 1px dashed rgba(205, 214, 244, 0.2);
}
.keyframe-text {
  flex: 1 1 auto;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.keyframe-card-caption.is-expanded .keyframe-text { white-space: normal; }
.keyframe-text-blind { color: var(--vp-c-text-3); font-style: italic; }
.keyframe-toggle {
  all: unset;
  box-sizing: border-box;
  display: block;
  width: 100%;
  padding: 6px 12px;
  font-size: 11px;
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  color: var(--vp-c-text-2);
  background: var(--vp-c-bg-mute);
  border-top: 1px solid var(--vp-divider);
  text-align: center;
  cursor: pointer;
  transition: background 120ms ease, color 120ms ease;
}
.keyframe-toggle:hover { background: var(--vp-c-bg-elv); color: var(--vp-c-text-1); }
.keyframe-toggle:focus-visible {
  outline: 2px solid var(--color-accent, #89b4fa);
  outline-offset: -2px;
}
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
