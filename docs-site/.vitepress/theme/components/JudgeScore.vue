<template>
  <div class="judge-score">
    <div class="score-bar">
      <div
        class="score-fill"
        :class="scoreClass"
        :style="{ width: `${scorePercent}%` }"
      ></div>
    </div>
    <div class="score-label" :class="scoreClass">
      {{ scorePercent }}%
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'

const props = withDefaults(
  defineProps<{
    score?: number
  }>(),
  {
    score: 0
  }
)

const scorePercent = computed(() => {
  return Math.min(100, Math.max(0, Math.round(props.score * 100)))
})

const scoreClass = computed(() => {
  const percent = scorePercent.value
  if (percent >= 70) return 'pass'
  if (percent >= 40) return 'warn'
  return 'fail'
})
</script>

<style scoped>
.judge-score {
  display: flex;
  align-items: center;
  gap: 8px;
}

.score-bar {
  flex: 1;
  height: 6px;
  background-color: var(--vp-c-bg-mute);
  border-radius: 3px;
  overflow: hidden;
  min-width: 60px;
}

.score-fill {
  height: 100%;
  border-radius: 3px;
  transition: width 0.3s ease;
}

.score-fill.pass {
  background-color: #10b981;
}

.score-fill.warn {
  background-color: #f59e0b;
}

.score-fill.fail {
  background-color: #ef4444;
}

.score-label {
  font-family: 'Monaco', 'Courier New', monospace;
  font-size: 12px;
  font-weight: 600;
  min-width: 40px;
  text-align: right;
}

.score-label.pass {
  color: #10b981;
}

.score-label.warn {
  color: #f59e0b;
}

.score-label.fail {
  color: #ef4444;
}
</style>
