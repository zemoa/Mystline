<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue"
import { invoke } from "@tauri-apps/api/core"
import { listen, type UnlistenFn } from "@tauri-apps/api/event"
import Icon from "./Icon.vue"
import SyntaxHelp from "./SyntaxHelp.vue"
import { version as appVersion } from "../../src-tauri/tauri.conf.json"

type Surface = "capture" | "panel"
interface Source { generation: number; revision: number }
interface UpdateStatus { sequence: number; phase: string; version: string | null; message: string }
interface DesktopStatus { version: number; tasksFile: string; taskCount: number; revision: number; today: string; autostart: boolean; captureShortcut: string; panelShortcut: string; captureActive: boolean; panelActive: boolean; repositoryError: string | null }
interface Row { position: number; section: "overdue" | "current" | "nextWorkday" | "later"; title: string; tags: string[]; planned: string | null; deadline: string | null; deadlineTone: string | null; completed: boolean; completedDate: string | null; editDraft: string }
interface TaskView { version: number; source: Source; today: string; nextWorkday: string; busy: boolean; rows: Row[] }
interface Editor { position: number; source: Source; draft: string; invalidated: boolean }
const params = new URLSearchParams(location.search)
const requestedSurface = params.get("surface")
const surface: Surface = requestedSurface === "capture" ? requestedSurface : "panel"
const session = Number(params.get("session"))
const status = ref<DesktopStatus | null>(null)
const updateStatus = ref<UpdateStatus | null>(null)
const updatePending = ref(false)
const view = ref<TaskView | null>(null)
const error = ref("")
const success = ref("")
const draft = ref("")
const query = ref("")
const tag = ref<string | null>(null)
const history = ref(false)
const searching = ref(false)
const adding = ref(false)
const help = ref(false)
const editor = ref<Editor | null>(null)
const pending = ref(false)
const settingsPending = ref(false)
const settingsView = ref(false)
const selected = ref<number | null>(null)
const captureInput = ref<HTMLInputElement | null>(null)
const editorInput = ref<HTMLInputElement | null>(null)
const searchInput = ref<HTMLInputElement | null>(null)
const list = ref<HTMLElement | null>(null)
const captureContent = ref<HTMLElement | null>(null)
const filePath = ref("")
const captureShortcut = ref("")
const panelShortcut = ref("")
const recovered = ref<{ draft: string; error: string }[]>([])
const canRecover = computed(() => !pending.value && !editor.value && !adding.value && !draft.value)
const unlisteners: UnlistenFn[] = []
let disposed = false
let activation = 0
let visible = true
let requests = 0
let resizeObserver: ResizeObserver | null = null
const request = () => ({ query: query.value, tag: tag.value, history: history.value })
const sameSource = (left: Source, right: Source) => left.generation === right.generation && left.revision === right.revision
const groups = computed(() => history.value ? [{ key: "history", label: "Tâches terminées", hint: "", rows: view.value?.rows || [] }] : [
  { key: "overdue", label: "En retard", hint: "Deadline dépassée." },
  { key: "current", label: "En cours", hint: "Sans date prévue · prévu aujourd’hui ou auparavant." },
  { key: "nextWorkday", label: "Prochain jour ouvré · " + dateLabel(view.value?.nextWorkday || "", true), hint: "" },
  { key: "later", label: "Plus tard", hint: "Autres dates prévues futures." },
].map(group => ({ ...group, rows: (view.value?.rows || []).filter(row => row.section === group.key) })))
const panelDate = computed(() => {
  const today = view.value?.today || status.value?.today
  return today ? new Date(today + "T12:00:00").toLocaleDateString("fr-FR", { weekday: "long", day: "numeric", month: "long" }) : ""
})
function dateLabel(date: string, weekday = false) {
  if (!date) return ""
  return new Date(date + "T12:00:00").toLocaleDateString("fr-FR", { ...(weekday ? { weekday: "long" as const } : {}), day: "numeric", month: "short" })
}
function sectionCount(rows: Row[]) {
  const completed = rows.filter(row => row.completed).length
  const done = `${completed} terminée${completed > 1 ? "s" : ""}`
  return history.value ? done : `${rows.length - completed} à faire${completed ? " · " + done : ""}`
}
function canReorder(row: Row, down: boolean) {
  const rows = (view.value?.rows || []).filter(other => !other.completed && other.section === row.section)
  const index = rows.findIndex(other => other.position === row.position)
  return index >= 0 && (down ? index < rows.length - 1 : index > 0)
}
function acceptStatus(value: DesktopStatus) {
  if (!status.value || value.version >= status.value.version) status.value = value
}
function syncSettings() {
  if (!status.value) return
  filePath.value = status.value.tasksFile
  captureShortcut.value = status.value.captureShortcut
  panelShortcut.value = status.value.panelShortcut
}
function acceptUpdate(value: UpdateStatus | null) {
  if (value && (!updateStatus.value || value.sequence >= updateStatus.value.sequence)) updateStatus.value = value
}
async function updateAction(command: "update_check" | "update_install") {
  if (updatePending.value) return
  updatePending.value = true
  try { await invoke(command) }
  catch (cause) { error.value = String(cause) }
  finally { updatePending.value = false }
}
async function refresh() {
  if (surface !== "panel" || settingsView.value || disposed) return
  const sequence = ++requests
  try {
    const latest = await invoke<TaskView>("task_view", { request: request() })
    if (disposed || sequence !== requests || (view.value && latest.version < view.value.version)) return
    const previous = view.value
    view.value = latest
    if (editor.value && !pending.value && !sameSource(editor.value.source, latest.source)) {
      editor.value.invalidated = true
      error.value = "La liste a changé. Conservez ce texte si nécessaire, puis Échap et rouvrez la tâche."
    }
    if (previous && previous.source.generation !== latest.source.generation) selected.value = null
    if (!latest.rows.some(row => row.position === selected.value)) selected.value = latest.rows[0]?.position ?? null
  } catch (cause) { if (!disposed && sequence === requests) error.value = String(cause) }
}
function resetTransientState() {
  draft.value = ""; query.value = ""; tag.value = null; history.value = false
  searching.value = false; adding.value = false; help.value = false; editor.value = null
  error.value = ""; success.value = ""; selected.value = null; pending.value = false
}
async function opened(event: { activation: number; reset: boolean; settings?: boolean }) {
  activation = event.activation
  visible = true
  settingsView.value = event.settings ?? false
  if (event.reset) { resetTransientState(); syncSettings() }
  await refresh()
  await nextTick()
  if (surface === "capture" || adding.value) captureInput.value?.focus()
  else if (editor.value) editorInput.value?.focus()
  else if (searching.value) searchInput.value?.focus()
  else if (surface === "panel") list.value?.focus()
}
function retainListener(stop: UnlistenFn) { if (disposed) stop(); else unlisteners.push(stop) }
async function run(command: string, args?: Record<string, unknown>): Promise<boolean> {
  try { if (args) await invoke(command, args); else await invoke(command); return true }
  catch (cause) { error.value = String(cause); return false }
}
function open(target: Surface | "settings") {
  void run(target === "settings" ? "open_settings" : "open_surface", target === "settings" ? undefined : { surface: target })
}
// Une action explicite de l'utilisateur ferme la surface actuellement affichée.
// La fermeture identifiée reste réservée aux retours asynchrones (par exemple
// après l'enregistrement d'une capture), qui ne doivent jamais fermer une
// surface rouverte entre-temps.
function closeCurrentSurface() { void run("close_current_surface") }
function closeAfterCapture() { void run("close_surface", { session, activation }) }
async function focusList() { await nextTick(); list.value?.focus(); scrollSelection() }
function scrollSelection() {
  void nextTick(() => list.value?.querySelector<HTMLElement>(`[data-position="${selected.value}"]`)?.scrollIntoView?.({ block: "nearest" }))
}
async function beginAdd() {
  if (editor.value || history.value) return
  adding.value = true; error.value = ""; help.value = false
  await nextTick(); captureInput.value?.focus()
}
async function beginEdit(row: Row) {
  if (row.completed || history.value || adding.value || pending.value || view.value?.busy || !view.value) return
  selected.value = row.position
  editor.value = { position: row.position, source: { ...view.value.source }, draft: row.editDraft, invalidated: false }
  error.value = ""; help.value = false; searching.value = false
  await nextTick(); editorInput.value?.focus()
}
async function beginSearch() {
  if (editor.value || adding.value) return
  searching.value = true
  await nextTick(); searchInput.value?.focus()
}
function toggleHistory() {
  if (editor.value || adding.value) return
  history.value = !history.value; searching.value = false; error.value = ""
  void focusList()
}
async function submitCreate() {
  if (pending.value) return
  const original = draft.value
  const submittedActivation = activation
  pending.value = true; error.value = ""
  try {
    await invoke("task_create", { draft: original, session, activation: submittedActivation })
    if (activation === submittedActivation) {
      draft.value = ""; adding.value = false; help.value = false
      if (surface === "capture") closeAfterCapture(); else await focusList()
    }
  } catch (cause) {
    if (activation === submittedActivation && visible) { draft.value = original; error.value = String(cause) }
    else recovered.value.push({ draft: original, error: String(cause) })
  } finally {
    if (activation === submittedActivation) pending.value = false
    await refresh()
  }
}
function recoverDraft() {
  if (!canRecover.value) return
  const failed = recovered.value.shift()
  if (failed) { draft.value = failed.draft; error.value = failed.error; if (surface === "panel") adding.value = true; void nextTick(() => captureInput.value?.focus()) }
}
async function submitEdit() {
  const submittedActivation = activation
  const editing = editor.value
  if (!editing || editing.invalidated || pending.value) return
  pending.value = true; error.value = ""
  try {
    await invoke("task_update", { source: editing.source, position: editing.position, draft: editing.draft })
    if (activation === submittedActivation && editor.value === editing) { editor.value = null; help.value = false; await focusList() }
  } catch (cause) { if (activation === submittedActivation) error.value = String(cause) }
  finally { if (activation === submittedActivation) { pending.value = false; await refresh() } }
}
async function toggleTask(row: Row) {
  const submittedActivation = activation
  if (!view.value || history.value || pending.value || view.value.busy || editor.value || adding.value) return
  pending.value = true; error.value = ""
  try { await invoke("task_toggle", { source: view.value.source, position: row.position, request: request() }) }
  catch (cause) { if (activation === submittedActivation) error.value = String(cause) }
  finally { if (activation === submittedActivation) { pending.value = false; await refresh(); scrollSelection() } }
}
async function reorder(down: boolean) {
  const submittedActivation = activation
  if (!view.value || selected.value === null || history.value || pending.value || view.value.busy || editor.value || adding.value) return
  const row = view.value.rows.find(row => row.position === selected.value)
  if (!row || row.completed || !canReorder(row, down)) return
  pending.value = true; error.value = ""
  try {
    const target = await invoke<number | null>("task_reorder", { source: view.value.source, position: selected.value, down, request: request() })
    if (activation === submittedActivation && target !== null) selected.value = target
  } catch (cause) { if (activation === submittedActivation) error.value = String(cause) }
  finally { if (activation === submittedActivation) { pending.value = false; await refresh(); await focusList() } }
}
function keydown(event: KeyboardEvent) {
  if (event.isComposing) return
  const target = event.target as HTMLElement | null
  const field = target?.tagName === "INPUT" || target?.tagName === "TEXTAREA"
  if (event.key === "F1" && (surface === "capture" || !settingsView.value)) { event.preventDefault(); help.value = !help.value; return }
  if (event.key === "Escape") {
    event.preventDefault()
    if (help.value) help.value = false
    else if (editor.value) { if (pending.value) return; editor.value = null; error.value = ""; void focusList() }
    else if (adding.value) { if (pending.value) return; adding.value = false; draft.value = ""; error.value = ""; void focusList() }
    else if (searching.value || query.value) { searching.value = false; query.value = ""; void focusList() }
    else if (history.value) { history.value = false; void focusList() }
    else closeCurrentSurface()
    return
  }
  if (surface !== "panel" || settingsView.value) return
  if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "f") { event.preventDefault(); void beginSearch(); return }
  if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "h") { event.preventDefault(); toggleHistory(); return }
  if (field || target?.closest("button") || editor.value || adding.value) return
  const rows = view.value?.rows || []
  const at = rows.findIndex(row => row.position === selected.value)
  if (event.key === "ArrowUp" || event.key === "ArrowDown") {
    event.preventDefault()
    if (event.ctrlKey) void reorder(event.key === "ArrowDown")
    else {
      const next = Math.min(rows.length - 1, Math.max(0, at + (event.key === "ArrowDown" ? 1 : -1)))
      selected.value = rows[next]?.position ?? null; scrollSelection()
    }
  } else if (event.key === "Enter" || event.key.toLowerCase() === "e") {
    const row = rows[at]; if (row) { event.preventDefault(); void beginEdit(row) }
  } else if (event.key === " " && rows[at]) { event.preventDefault(); void toggleTask(rows[at]) }
}
async function setting(command: string, args: Record<string, unknown>, message: string) {
  const submittedActivation = activation
  if (settingsPending.value) return
  settingsPending.value = true; error.value = ""; success.value = ""
  try {
    await invoke(command, args)
    acceptStatus(await invoke<DesktopStatus>("desktop_status"))
    if (activation === submittedActivation) success.value = message
  } catch (cause) { if (activation === submittedActivation) error.value = String(cause) }
  finally { settingsPending.value = false }
}
async function chooseSource(create: boolean) {
  if (settingsPending.value) return
  const submittedActivation = activation
  settingsPending.value = true
  error.value = ""
  try {
    const path = await invoke<string | null>("settings_choose", { create })
    if (activation !== submittedActivation) return
    if (path) { filePath.value = path; settingsPending.value = false; await setting("settings_source", { path, create }, "Fichier de tâches activé.") }
  } catch (cause) { if (activation === submittedActivation) error.value = String(cause) }
  finally { settingsPending.value = false }
}
function tagClass(value: string) { let hash = 0; for (const char of value.toLowerCase()) hash = (hash * 31 + char.charCodeAt(0)) | 0; return `tag-${Math.abs(hash) % 4}` }
async function resizeCapture() { if (surface === "capture") { await nextTick(); await run("capture_layout", { height: (captureContent.value?.scrollHeight || 120) + 32 }) } }
watch([query, tag, history], () => { void refresh() })
watch([help, error, recovered], () => { void resizeCapture() }, { deep: true })
onMounted(async () => {
  window.addEventListener("keydown", keydown)
  try {
    retainListener(await listen<{ activation: number; reset: boolean }>("surface-opened", event => { void opened(event.payload) }))
    retainListener(await listen<number>("surface-closed", event => { if (event.payload === activation) { visible = false; resetTransientState() } }))
    retainListener(await listen<DesktopStatus>("desktop-status", event => { acceptStatus(event.payload) }))
    retainListener(await listen("task-changed", () => { void refresh() }))
    retainListener(await listen<UpdateStatus>("update-status", event => { acceptUpdate(event.payload) }))
    if (disposed) return
    acceptStatus(await invoke<DesktopStatus>("desktop_status")); syncSettings()
    acceptUpdate(await invoke<UpdateStatus>("update_status"))
    await refresh()
  } catch (cause) { error.value = String(cause) }
  await nextTick()
  await run("surface_ready", { session })
  if (surface === "capture") {
    captureInput.value?.focus()
    if (typeof ResizeObserver !== "undefined" && captureContent.value) { resizeObserver = new ResizeObserver(() => { void resizeCapture() }); resizeObserver.observe(captureContent.value) }
  }
})
onUnmounted(() => { disposed = true; for (const stop of unlisteners) stop(); resizeObserver?.disconnect(); window.removeEventListener("keydown", keydown) })
</script>

<template>
  <main :class="['surface', surface]">
    <div v-if="surface === 'capture'" ref="captureContent" class="capture-content">
      <div class="capture-top"><span class="surface-label">Capture rapide</span><button type="button" aria-label="Fermer" @click="closeCurrentSurface"><Icon name="close" /></button></div>
      <form @submit.prevent="submitCreate">
        <label class="sr-only" for='capture'>Saisie de la tâche</label>
        <input id='capture' ref="captureInput" v-model="draft" :readonly="pending" placeholder="Que souhaitez-vous faire ?" autocomplete="off" />
        <p class="muted hints">{{ pending ? "Enregistrement…" : "Entrée pour ajouter · Échap pour fermer · F1 pour l’aide" }}</p>
      </form>
      <SyntaxHelp v-if="help" />
      <p v-if="error" role="alert" class="error">{{ error }}</p>
      <button v-if="recovered.length" class="secondary" :disabled="!canRecover" @click="recoverDraft">Récupérer une saisie non enregistrée</button>
    </div>

    <template v-else>
      <header>
        <div class="brand"><img class="brand-mark" src="/mystline-mark.svg" alt="" aria-hidden="true" /><span>Mystline</span></div>
        <nav aria-label="Actions">
          <button v-if="settingsView" aria-label="Retour aux tâches" @click="open('panel')"><Icon name="back" /></button>
          <button v-if="!settingsView && history" aria-label="Retour aux tâches" @click="toggleHistory"><Icon name="back" /></button>
          <button v-if="!settingsView" aria-label="Rechercher" title="Rechercher · Ctrl+F" @click="beginSearch"><Icon name="search" /></button>
          <button v-if="!settingsView && !history" aria-label="Historique" title="Historique · Ctrl+H" @click="toggleHistory"><Icon name="history" /></button>
          <button v-if="!settingsView" aria-label="Ouvrir les paramètres" @click="open('settings')"><Icon name='settings' /></button>
          <button type="button" aria-label="Fermer" @click="closeCurrentSurface"><Icon name="close" /></button>
        </nav>
      </header>
      <div class="page-heading">
        <h1>{{ settingsView ? "Paramètres" : history ? "Historique" : "Mes tâches" }}</h1>
        <p v-if="!settingsView && panelDate" class="panel-date">{{ panelDate }}</p>
      </div>
      <p v-if="error || status?.repositoryError" role="alert" class="error">{{ error || status?.repositoryError }}</p>
      <p v-if="success" role="status" class="success">{{ success }}</p>
      <template v-if="!settingsView">
        <div v-if="searching || query" class="search-bar"><label class="sr-only" for="search">Rechercher dans le titre et les tags</label><input id="search" ref="searchInput" v-model="query" placeholder="Rechercher un titre ou un tag" @keydown.enter.prevent="searching = false; focusList()" /><button aria-label="Fermer la recherche" @click="query = ''; searching = false; focusList()"><Icon name="close" /></button></div>
        <div v-if='tag' class="filter"><span>Filtre : #{{ tag }}</span><button class="secondary" @click="tag = null">Retirer le filtre</button></div>
        <SyntaxHelp v-if="help && !editor && !adding" />
        <form v-if="editor" class="composer" @submit.prevent="submitEdit">
          <label for="edit">Modifier la tâche</label><input id="edit" ref="editorInput" v-model="editor.draft" :readonly="pending" autocomplete="off" />
          <SyntaxHelp v-if="help" /><p class="muted">Entrée pour enregistrer · Échap pour annuler · F1 pour l’aide</p>
          <div class="form-actions"><button type="button" class="secondary" :disabled="pending" @click="editor = null; help = false; focusList()">Annuler</button><button class="primary" :disabled="pending || editor.invalidated">{{ pending ? "Enregistrement…" : "Enregistrer" }}</button></div>
        </form>
        <form v-if="adding" class="composer" @submit.prevent="submitCreate">
          <label for='capture'>Ajouter une tâche</label><input id='capture' ref="captureInput" v-model="draft" :readonly="pending" placeholder="Une tâche, simplement" autocomplete="off" />
          <SyntaxHelp v-if="help" /><p class="muted">Entrée pour ajouter · Échap pour annuler · F1 pour l’aide</p><button class="primary" :disabled="pending">{{ pending ? "Enregistrement…" : "Ajouter" }}</button>
        </form>
        <div ref="list" class="task-list" tabindex="0" role="list" aria-label="Tâches">
          <section v-for="group in groups" :key="group.key" :class="['task-section', group.key]">
            <h2>{{ group.label }}<span>{{ sectionCount(group.rows) }}</span></h2>
            <p v-if="group.hint" class="section-hint">{{ group.hint }}</p>
            <p v-if="!group.rows.length" class="empty">{{ query || tag ? "Aucun résultat" : history ? "Aucune tâche terminée" : "Aucune tâche" }}</p>
            <article v-for="row in group.rows" :key="row.position" role="listitem" :data-position="row.position" :class="['task', { selected: selected === row.position, completed: row.completed }]" @click="selected = row.position">
              <input v-if="!history" type="checkbox" :checked="row.completed" :disabled="pending || view?.busy || !!editor || adding" :aria-label="(row.completed ? 'Réouvrir : ' : 'Terminer : ') + row.title" @change="toggleTask(row)" />
              <Icon v-else name="check" />
              <div class="task-body">
                <button v-if="!history && !row.completed" class="task-title" @click="beginEdit(row)">{{ row.title }}</button><span v-else class="task-title">{{ row.title }}</span>
                <div v-if="row.tags.length || row.planned || row.deadline || row.completed" class="task-details">
                  <button v-for="item in row.tags" :key="item" :class="['tag', tagClass(item)]" :aria-label="'Filtrer par #' + item" @click.stop="tag = item">#{{ item }}</button>
                  <span v-if="history" class='date'>Terminée {{ dateLabel(row.completedDate || '') }}</span>
                  <span v-else-if="row.planned" class='date'>{{ row.planned === view?.today ? "Prévu aujourd’hui" : "Prévu le " + dateLabel(row.planned) }}</span>
                  <span v-if="row.deadline" :class="['deadline', row.deadlineTone]">Échéance {{ !row.completed && row.deadline === view?.today ? "aujourd’hui" : dateLabel(row.deadline) }}</span>
                  <span v-if="!history && row.completed" class="date">Terminée aujourd’hui</span>
                </div>
                <div v-if="!history && !row.completed && selected === row.position && !editor && !adding" class="row-actions">
                  <button :disabled="pending || view?.busy || !canReorder(row, false)" :aria-label="'Monter : ' + row.title" @click.stop="reorder(false)">↑ Monter</button>
                  <button :disabled="pending || view?.busy || !canReorder(row, true)" :aria-label="'Descendre : ' + row.title" @click.stop="reorder(true)">↓ Descendre</button>
                </div>
              </div>
              <button v-if="!history && !row.completed" class="edit-action" :aria-label="'Modifier : ' + row.title" @click="beginEdit(row)"><Icon name="edit" /></button>
            </article>
          </section>
        </div>
        <footer><button v-if="!history" class="primary add-task" :disabled="adding || !!editor" @click="beginAdd"><Icon name="add" />Ajouter une tâche</button><span v-if="view?.busy" class="muted" role="status">Enregistrement…</span><span v-else class="muted">↑ ↓ naviguer · Espace cocher · F1 aide</span></footer>
        <button v-if="recovered.length" class="secondary" :disabled="!canRecover" @click="recoverDraft">Récupérer une saisie non enregistrée</button>
      </template>
      <div v-else class="settings-body">
        <section class="setting-section"><h2>Fichier de tâches</h2><p class="muted path">Source actuelle : {{ status?.tasksFile }}</p>
          <form @submit.prevent="setting('settings_source', { path: filePath, create: false }, 'Fichier de tâches activé.')"><label for="file-path">Chemin du fichier</label><input id="file-path" v-model="filePath" autocomplete="off" /><div class="form-actions"><button class="secondary" :disabled="settingsPending">Utiliser ce fichier</button><button type="button" class="secondary" :disabled="settingsPending" @click="setting('settings_source', { path: filePath, create: true }, 'Fichier de tâches activé.')">Créer à ce chemin</button></div></form>
          <div class="form-actions"><button class="secondary" :disabled="settingsPending" @click="chooseSource(false)">Choisir un fichier…</button><button class="secondary" :disabled="settingsPending" @click="chooseSource(true)">Nouveau fichier…</button></div>
        </section>
        <section class="setting-section"><h2>Démarrage</h2><label class="check-setting"><input type="checkbox" :checked="status?.autostart" :disabled="settingsPending" @change="setting('settings_autostart', { enabled: ($event.target as HTMLInputElement).checked }, 'Démarrage automatique mis à jour.')" />Démarrer avec la session</label></section>
        <section class="setting-section"><h2>Raccourcis globaux</h2>
          <form @submit.prevent="setting('settings_shortcut', { capture: true, value: captureShortcut }, 'Raccourci de capture enregistré.')"><label for="capture-shortcut">Capture rapide <span :class="['badge', { active: status?.captureActive }]">{{ status?.captureActive ? "Actif" : "Inactif" }}</span></label><div class="setting-input"><input id="capture-shortcut" v-model="captureShortcut" autocomplete="off" /><button class="secondary" :disabled="settingsPending">Enregistrer</button></div></form>
          <form @submit.prevent="setting('settings_shortcut', { capture: false, value: panelShortcut }, 'Raccourci du panneau enregistré.')"><label for="panel-shortcut">Ouvrir / masquer le panneau <span :class="['badge', { active: status?.panelActive }]">{{ status?.panelActive ? "Actif" : "Inactif" }}</span></label><div class="setting-input"><input id="panel-shortcut" v-model="panelShortcut" autocomplete="off" /><button class="secondary" :disabled="settingsPending">Enregistrer</button></div></form>
          <p class="muted">Exemple : Ctrl+Shift+Espace. Si un raccourci est inactif, utilisez l’icône de la zone de notification.</p>
        </section>
        <section class="setting-section"><h2>Mises à jour</h2>
          <p class="muted">Mystline · Version {{ appVersion }}</p>
          <p role="status">{{ updateStatus?.message }}</p>
          <div v-if="updateStatus && updateStatus.phase !== 'disabled'" class="form-actions">
            <button v-if="updateStatus.phase === 'ready'" class="primary" :disabled="updatePending || settingsPending || pending || view?.busy || recovered.length > 0" @click="updateAction('update_install')">Installer et redémarrer</button>
            <button v-else class="secondary" :disabled="updatePending || ['checking', 'downloading'].includes(updateStatus.phase)" @click="updateAction('update_check')">Vérifier les mises à jour</button>
          </div>
        </section>
      </div>
    </template>
  </main>
</template>
