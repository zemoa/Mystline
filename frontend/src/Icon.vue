<script setup lang="ts">
defineProps<{ name: string }>()
const paths: Record<string, string> = {
  add: 'M12 5v14M5 12h14', close: 'm6 6 12 12M18 6 6 18',
  search: 'M10.5 3a7.5 7.5 0 1 0 0 15 7.5 7.5 0 0 0 0-15ZM16 16l5 5',
  history: 'M3 11a9 9 0 1 1 3 8M3 4v7h7M12 7v5l3 2',
  settings: 'm9 3-.5 2-2 1-2-.5-2 3 1.5 1.5v3L2.5 15l2 3 2-.5 2 1L9 21h6l.5-2.5 2-1 2 .5 2-3-1.5-2v-3L21.5 8l-2-3-2 .5-2-1L15 3ZM12 9a3 3 0 1 0 0 6 3 3 0 0 0 0-6Z',
  back: 'M20 12H4m7-7-7 7 7 7', up: 'M12 20V4m-6 6 6-6 6 6', down: 'M12 4v16m-6-6 6 6 6-6',
  edit: 'm16 3 5 5-12 12H4v-5ZM13 6l5 5', check: 'm5 12 4 4 10-10',
}
</script>
<template><svg viewBox="0 0 24 24" aria-hidden="true"><path :d="paths[name] || paths.add" /></svg></template>
