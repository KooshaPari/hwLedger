<template>
  <tr>
    <td class="step-index">{{ index + 1 }}</td>
    <td class="step-slug">{{ step.slug }}</td>
    <td class="step-intent">{{ step.intent }}</td>
    <td>
      <div v-if="step.screenshot" class="step-thumbnail">
        <img :src="step.screenshot" :alt="step.slug" />
      </div>
      <span v-else style="color: var(--vp-c-text-3);">-</span>
    </td>
    <td class="step-description">
      {{ step.description || 'No verification' }}
    </td>
    <td>
      <JudgeScore :score="step.judge_score || 0" />
    </td>
  </tr>
</template>

<script setup lang="ts">
import JudgeScore from './JudgeScore.vue'

interface JourneyStep {
  slug: string
  intent: string
  screenshot?: string
  description?: string
  judge_score?: number
}

defineProps<{
  step: JourneyStep
  index: number
}>()
</script>

<style scoped>
tr {
  border-bottom: 1px solid var(--vp-divider);
}

tr:hover {
  background-color: var(--vp-c-bg-soft);
}

td {
  padding: 12px;
  font-size: 13px;
}

.step-index {
  font-family: 'Monaco', 'Courier New', monospace;
  font-size: 12px;
  color: var(--vp-c-text-3);
  font-weight: 600;
}

.step-slug {
  font-family: 'Monaco', 'Courier New', monospace;
  color: var(--color-accent);
}

.step-intent {
  font-weight: 500;
}

.step-thumbnail {
  width: 48px;
  height: 27px;
  background-color: var(--vp-c-bg-mute);
  border-radius: 4px;
  display: inline-block;
  overflow: hidden;
  border: 1px solid var(--vp-divider);
}

.step-thumbnail img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.step-description {
  color: var(--vp-c-text-2);
  max-width: 300px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
