<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

type Surface = 'capture' | 'panel' | 'settings'
interface DesktopStatus {
  tasksFile: string
  taskCount: number
  revision: number
  today: string
  autostart: boolean
  captureShortcut: string
  panelShortcut: string
  captureActive: boolean
  panelActive: boolean
  repositoryError: string | null
}

const params = new URLSearchParams(location.search)
const requestedSurface = params.get('surface')
const surface: Surface = requestedSurface === 'capture' || requestedSurface === 'settings' ? requestedSurface : 'panel'
const session = Number(params.get('session'))
const status = ref<DesktopStatus | null>(null)
const error = ref('')
const draft = ref('')
const captureInput = ref<HTMLInputElement | null>(null)
const unlisteners: UnlistenFn[] = []
let disposed = false
let activation = 0

function resetTransientState() { draft.value = ''; error.value = '' }
async function opened(event: { activation: number; reset: boolean }) {
  activation = event.activation
  if (event.reset) resetTransientState()
  await nextTick()
  if (surface === 'capture') captureInput.value?.focus()
}
function retainListener(stop: UnlistenFn) {
  if (disposed) stop()
  else unlisteners.push(stop)
}

async function run(command: string, args?: Record<string, unknown>) {
  try { await invoke(command, args) }
  catch (cause) { error.value = String(cause) }
}
function open(target: Surface) { void run('open_surface', { surface: target }) }
function close() { void run('close_surface') }
function keydown(event: KeyboardEvent) {
  if (event.key === 'Escape') { event.preventDefault(); close() }
}

onMounted(async () => {
  window.addEventListener('keydown', keydown)
  try {
    retainListener(await listen<{ activation: number; reset: boolean }>('surface-opened', event => { void opened(event.payload) }))
    retainListener(await listen<number>('surface-closed', event => { if (event.payload === activation) resetTransientState() }))
    retainListener(await listen<DesktopStatus>('desktop-status', event => { status.value = event.payload }))
    if (disposed) return
    status.value = await invoke<DesktopStatus>('desktop_status')
  } catch (cause) { error.value = String(cause) }
  // Toujours afficher la surface, y compris lorsqu'un diagnostic échoue.
  await nextTick()
  await run('surface_ready', { session })
  if (surface === 'capture') captureInput.value?.focus()
})
onUnmounted(() => {
  disposed = true
  for (const stop of unlisteners) stop()
  window.removeEventListener('keydown', keydown)
})
</script>

<template>
  <main :class="['surface', surface]">
    <header>
      <h1>{{ surface === 'capture' ? 'Capture rapide' : surface === 'settings' ? 'Paramètres' : 'Mes tâches' }}</h1>
      <nav aria-label="Actions">
        <button v-if="surface === 'panel'" aria-label="Ouvrir la capture" @click="open('capture')">+</button>
        <button v-if="surface === 'panel'" aria-label="Ouvrir les paramètres" @click="open('settings')">
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d="m9 3-.5 2-2 1-2-.5-2 3 1.5 1.5v3L2.5 15l2 3 2-.5 2 1L9 21h6l.5-2.5 2-1 2 .5 2-3-1.5-2v-3L21.5 8l-2-3-2 .5-2-1L15 3Z"/><circle cx="12" cy="12" r="3"/></svg>
        </button>
        <button aria-label="Fermer" @click="close()">
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d="m6 6 12 12M18 6 6 18"/></svg>
        </button>
      </nav>
    </header>

    <p v-if="error || status?.repositoryError" role="alert" class="error">{{ error || status?.repositoryError }}</p>

    <template v-if="surface === 'capture'">
      <label class="sr-only" for="capture">Saisie</label>
      <input id="capture" ref="captureInput" v-model="draft" placeholder="Tester la saisie au clavier" autocomplete="off" />
      <p class="muted">Échap pour fermer · L’enregistrement sera disponible au prochain lot.</p>
    </template>

    <template v-else-if="surface === 'panel'">
      <div class="notice">
        <p v-if="status">{{ status.taskCount }} tâche{{ status.taskCount === 1 ? '' : 's' }} dans votre fichier.</p>
        <p>La liste et les actions seront disponibles dans les prochains lots.</p>
      </div>
      <button class="secondary" @click="open('capture')">Ouvrir la capture rapide</button>
    </template>

    <template v-else>
      <dl v-if="status">
        <dt>Fichier de tâches</dt><dd class="path">{{ status.tasksFile }}</dd>
        <dt>Démarrage avec la session</dt><dd>{{ status.autostart ? 'Activé' : 'Désactivé' }}</dd>
        <dt>Capture rapide</dt><dd>{{ status.captureShortcut }} <span :class="['badge', { active: status.captureActive }]">{{ status.captureActive ? 'Actif' : 'Inactif' }}</span></dd>
        <dt>Ouvrir / masquer le panneau</dt><dd>{{ status.panelShortcut }} <span :class="['badge', { active: status.panelActive }]">{{ status.panelActive ? 'Actif' : 'Inactif' }}</span></dd>
      </dl>
      <p class="muted">La modification des paramètres sera disponible dans un prochain lot.</p>
    </template>
  </main>
</template>
