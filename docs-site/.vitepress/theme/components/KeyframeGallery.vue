<template>
  <div class="keyframe-gallery">
    <div class="keyframe-carousel">
      <img
        v-if="currentKeyframe"
        :src="currentKeyframe.path"
        :alt="currentKeyframe.caption"
        class="keyframe-image"
      />
      <div class="carousel-controls">
        <button class="carousel-btn" @click="previousFrame" :disabled="currentIndex === 0">
          Previous
        </button>
        <span class="carousel-counter">
          {{ currentIndex + 1 }} / {{ keyframes.length }}
        </span>
        <button class="carousel-btn" @click="nextFrame" :disabled="currentIndex === keyframes.length - 1">
          Next
        </button>
      </div>
    </div>
    <div class="keyframe-caption">
      {{ currentKeyframe?.caption || 'Keyframe' }}
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'

interface Keyframe {
  path: string
  caption: string
}

const props = withDefaults(
  defineProps<{
    keyframes: Keyframe[]
    title?: string
  }>(),
  {
    keyframes: () => []
  }
)

const currentIndex = ref(0)

const currentKeyframe = computed(() => {
  return props.keyframes[currentIndex.value]
})

function nextFrame() {
  if (currentIndex.value < props.keyframes.length - 1) {
    currentIndex.value++
  }
}

function previousFrame() {
  if (currentIndex.value > 0) {
    currentIndex.value--
  }
}
</script>

<style scoped>
.keyframe-gallery {
  margin: 20px 0;
  border: 1px solid var(--vp-divider);
  border-radius: 8px;
  overflow: hidden;
}

.keyframe-carousel {
  position: relative;
  aspect-ratio: 16 / 9;
  background-color: var(--vp-c-bg-mute);
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
}

.keyframe-image {
  width: 100%;
  height: 100%;
  object-fit: contain;
  display: block;
}

.keyframe-caption {
  padding: 12px;
  background-color: var(--vp-c-bg-mute);
  font-size: 13px;
  color: var(--vp-c-text-2);
  border-top: 1px solid var(--vp-divider);
}

.carousel-controls {
  position: absolute;
  bottom: 12px;
  left: 50%;
  transform: translateX(-50%);
  display: flex;
  gap: 8px;
  background-color: rgba(0, 0, 0, 0.6);
  padding: 8px 12px;
  border-radius: 6px;
  z-index: 10;
}

.carousel-btn {
  background: transparent;
  border: 1px solid rgba(255, 255, 255, 0.3);
  color: white;
  padding: 4px 8px;
  cursor: pointer;
  border-radius: 4px;
  font-size: 12px;
  transition: all 0.2s ease;
}

.carousel-btn:hover:not(:disabled) {
  background-color: rgba(255, 255, 255, 0.1);
  border-color: rgba(255, 255, 255, 0.6);
}

.carousel-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.carousel-counter {
  color: rgba(255, 255, 255, 0.7);
  font-size: 12px;
  padding: 0 8px;
  align-self: center;
  white-space: nowrap;
}
</style>
