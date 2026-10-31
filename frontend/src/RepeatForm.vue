<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from "vue"
import { invoke } from "@tauri-apps/api/core"

const props = defineProps<{ draft: string; source?: { generation: number; revision: number }; position?: number }>()
const emit = defineEmits<{ apply: [draft: string]; cancel: [] }>()
type Preview = { draft: string; repeat: string | null; planned: string | null; deadline: string | null; dates: { planned: string; deadline: string | null }[] }
const weekdays = ["lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi", "dimanche"]
const frequency = ref("jour")
const frequencyInput = ref<HTMLSelectElement | null>(null)
const days = ref(["lundi"])
const interval = ref(30)
const monthDay = ref(31)
const planned = ref("")
const deadline = ref("")
const error = ref("")
const preview = ref<Preview | null>(null)
const loading = ref(false)
let sequence = 0
let ready = false
// Seules les commandes explicites non échappées sont remplacées ; le titre reste intact.
const base = computed(() => props.draft.replace(/(^|\s)\/(?:r|p|d)(?=\s|$)(?:\s+\S+)?/g, "$1").trim())
const rule = computed(() => frequency.value === "semaine" ? days.value.join(",") : frequency.value === "intervalle" ? `${interval.value}j` : frequency.value === "mois" ? `mois:${monthDay.value}` : "jour")
async function refresh() {
  const current = ++sequence
  preview.value = null; error.value = ""; loading.value = true
  const draft = `${base.value} /r ${rule.value}${planned.value ? ` /p ${planned.value}` : ""}${deadline.value ? ` /d ${deadline.value}` : ""}`
  try {
    const value = await invoke<Preview>("task_repeat_preview", { source: props.source ?? null, position: props.position ?? null, draft })
    if (current === sequence) { preview.value = value }
  } catch (cause) { if (current === sequence) error.value = String(cause) }
  finally { if (current === sequence) loading.value = false }
}
function changeFrequency() { planned.value = "" }
watch([frequency, days, interval, monthDay, planned, deadline], () => { if (ready) void refresh() }, { deep: true })
onMounted(async () => {
  await nextTick(); frequencyInput.value?.focus()
  try {
    const hasRepeat = /(^|\s)\/r\s/.test(props.draft)
    const value = await invoke<Preview>("task_repeat_preview", { source: props.source ?? null, position: props.position ?? null, draft: props.draft + (hasRepeat ? "" : " /r jour") })
    const repeat = value.repeat || "jour"
    if (repeat.startsWith("mois:")) { frequency.value = "mois"; monthDay.value = Number(repeat.slice(5)) }
    else if (/^\d+j$/.test(repeat)) { frequency.value = "intervalle"; interval.value = Number(repeat.slice(0, -1)) }
    else if (repeat !== "jour") { frequency.value = "semaine"; days.value = repeat.split(",") }
    planned.value = value.planned || ""; deadline.value = value.deadline || ""; preview.value = value
  } catch (cause) { error.value = String(cause) }
  finally { ready = true }
})
</script>

<template>
  <section class="repeat-form" role="dialog" aria-label="Répéter la tâche" @keydown.esc.stop.prevent="emit('cancel')" @keydown.enter.stop.prevent="preview && !loading && emit('apply', preview.draft)">
    <h2>Répéter la tâche</h2>
    <label for="repeat-frequency">Fréquence</label>
    <select id="repeat-frequency" ref="frequencyInput" v-model="frequency" @change="changeFrequency"><option value="jour">Tous les jours</option><option value="semaine">Chaque semaine</option><option value="intervalle">Tous les N jours</option><option value="mois">Chaque mois</option></select>
    <fieldset v-if="frequency === 'semaine'"><legend>Jours</legend><label v-for="day in weekdays" :key="day" class="check-setting"><input v-model="days" type="checkbox" :value="day" @change="changeFrequency" />{{ day }}</label></fieldset>
    <label v-if="frequency === 'intervalle'">Intervalle en jours<input v-model.number="interval" type="number" min="1" step="1" @input="changeFrequency" /></label>
    <label v-if="frequency === 'mois'">Jour du mois<input v-model.number="monthDay" type="number" min="1" max="31" step="1" @input="changeFrequency" /></label>
    <p v-if="frequency === 'mois'" class="muted">Dans un mois plus court, la tâche revient le dernier jour du mois.</p>
    <label for="repeat-planned">Première date</label><input id="repeat-planned" v-model="planned" type="date" :placeholder="preview?.planned || ''" />
    <p v-if="!planned && preview?.planned" class="muted">Date proposée : {{ preview.planned }}</p>
    <label for="repeat-deadline">Deadline (facultative)</label><input id="repeat-deadline" v-model="deadline" type="date" />
    <p class="muted">Modifier seulement /p dans la saisie reporte une occurrence. Ce formulaire règle le calendrier de répétition.</p>
    <p v-if="error" role="alert" class="error">{{ error }}</p>
    <template v-if="preview"><p>Dates du calendrier :</p><ol><li v-for="date in preview.dates" :key="date.planned">{{ date.planned }}<span v-if="date.deadline"> · deadline {{ date.deadline }}</span></li></ol><p class="muted">Une seule occurrence reste à faire. La répétition continue jusqu’à « Ne plus répéter ».</p></template>
    <div class="form-actions"><button type="button" class="secondary" @click="emit('cancel')">Annuler la répétition</button><button type="button" class="primary" :disabled="!preview || loading" @click="preview && emit('apply', preview.draft)">Utiliser cette répétition</button></div>
  </section>
</template>
