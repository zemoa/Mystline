import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import { deferred, desktop, emit, row, status, taskView } from './desktop.mock'
import App from '../src/App.vue'

let wrapper: VueWrapper | undefined

async function open(surface: 'capture' | 'panel' | 'settings') {
  window.history.replaceState({}, '', `/?surface=${surface}&session=3`)
  wrapper = mount(App, { attachTo: document.body })
  await flushPromises()
  emit('surface-opened', { activation: 10, reset: true, settings: surface === 'settings' })
  await flushPromises()
  return wrapper
}

function key(key: string, options: KeyboardEventInit = {}) {
  const event = new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true, ...options })
  document.activeElement!.dispatchEvent(event)
  return event
}

beforeEach(() => {
  desktop.listeners.clear()
  desktop.invoke.mockReset()
  desktop.invoke.mockImplementation(async (command: string) => {
    if (command === 'desktop_status') return { ...status }
    if (command === 'task_view') return taskView()
  })
})

afterEach(() => {
  wrapper?.unmount()
  wrapper = undefined
  document.body.innerHTML = ''
})

describe('surfaces Vue réutilisées', () => {
  it('ouvre et ferme l’aide depuis le panneau principal avec F1', async () => {
    // RG-F2-07 — Étant donné le panneau principal affichant les tâches.
    const app = await open('panel')
    // Quand F1 est pressée, puis Échap.
    key('F1'); await flushPromises()
    expect(app.get('[aria-label="Aide à la saisie"]').text()).toContain('/p')
    key('Escape'); await flushPromises()
    // Alors l’aide se ferme sans masquer le panneau.
    expect(app.find('[aria-label="Aide à la saisie"]').exists()).toBe(false)
    expect(desktop.invoke.mock.calls.filter(([command]) => command === 'close_current_surface')).toHaveLength(0)
  })

  it('focalise la capture, conserve le brouillon au focus et le retire lors de la fermeture courante', async () => {
    // F1/F5 — Étant donné une capture ouverte et son brouillon non enregistré.
    const app = await open('capture')
    const input = app.get<HTMLInputElement>('#capture')
    expect(document.activeElement).toBe(input.element)
    await input.setValue('Préparer le support #mission')
    // Quand le tray redonne le focus à la même activation.
    emit('surface-opened', { activation: 10, reset: false })
    await flushPromises()
    expect(input.element.value).toBe('Préparer le support #mission')
    // Alors la fermeture courante retire le brouillon, et l'ouverture suivante est vierge.
    emit('surface-closed', 10)
    await flushPromises()
    expect(input.element.value).toBe('')
    emit('surface-opened', { activation: 11, reset: true })
    await flushPromises()
    expect(document.activeElement).toBe(input.element)
    expect(desktop.invoke).toHaveBeenCalledWith('surface_ready', { session: 3 })
  })

  it('une fermeture retardée nefface pas une nouvelle activation et démonter retire les écouteurs', async () => {
    // F1/F5 — Étant donné une nouvelle activation portant un nouveau brouillon.
    const app = await open('capture')
    emit('surface-opened', { activation: 11, reset: true })
    await flushPromises()
    const input = app.get<HTMLInputElement>('#capture')
    await input.setValue('Nouveau brouillon')
    // Quand une fermeture issue de l'activation précédente arrive tardivement.
    emit('surface-closed', 10)
    await flushPromises()
    // Alors le brouillon actuel reste affiché et aucune fermeture IPC n'est déclenchée.
    expect(input.element.value).toBe('Nouveau brouillon')
    expect(desktop.invoke.mock.calls.filter(([command]) => command === 'close_surface')).toHaveLength(0)
    app.unmount()
    wrapper = undefined
    expect(desktop.listeners.size).toBe(0)
    key('Escape')
    expect(desktop.invoke.mock.calls.filter(([command]) => command === 'close_surface')).toHaveLength(0)
  })
})

describe('capture confirmée et erreurs', () => {
  it('attend la confirmation et ne soumet pas deux fois le même brouillon', async () => {
    // RG-F1-03/F0-09 — Étant donné une écriture dont la confirmation est différée.
    const confirmation = deferred<void>()
    desktop.invoke.mockImplementation(async command => command === 'desktop_status' ? status : command === 'task_create' ? confirmation.promise : undefined)
    const app = await open('capture')
    await app.get('#capture').setValue('Préparer /p demain #mission')
    // Quand le formulaire est validé deux fois avant la réponse du stockage.
    await app.get('form').trigger('submit')
    await app.get('form').trigger('submit')
    expect(desktop.invoke.mock.calls.filter(([command]) => command === 'task_create')).toHaveLength(1)
    expect(desktop.invoke).toHaveBeenCalledWith('task_create', { draft: 'Préparer /p demain #mission', session: 3, activation: 10 })
    expect(app.get<HTMLInputElement>('#capture').element.value).toBe('Préparer /p demain #mission')
    expect(desktop.invoke.mock.calls.filter(([command]) => command === 'close_surface')).toHaveLength(0)
    // Alors seule la confirmation autorise effacement et fermeture de cette activation.
    confirmation.resolve()
    await flushPromises()
    expect(app.get<HTMLInputElement>('#capture').element.value).toBe('')
    expect(desktop.invoke).toHaveBeenCalledWith('close_surface', { session: 3, activation: 10 })
  })

  it('conserve le brouillon refusé et ferme laide avant la capture', async () => {
    // RG-F2-04/07 — Étant donné une saisie refusée par le parseur ou le stockage.
    desktop.invoke.mockImplementation(async command => {
      if (command === 'desktop_status') return status
      if (command === 'task_create') throw new Error('Date impossible')
    })
    const app = await open('capture')
    await app.get('#capture').setValue('Titre /d impossible')
    await app.get('form').trigger('submit')
    await flushPromises()
    expect(app.get('[role=alert]').text()).toContain('Date impossible')
    expect(app.get<HTMLInputElement>('#capture').element.value).toBe('Titre /d impossible')
    // Quand F1 ouvre l'aide puis Échap la referme.
    key('F1'); await flushPromises()
    expect(app.text()).toContain('/p')
    key('Escape'); await flushPromises()
    // Alors le brouillon et l'erreur restent, sans commande de fermeture native.
    expect(app.get<HTMLInputElement>('#capture').element.value).toBe('Titre /d impossible')
    expect(desktop.invoke.mock.calls.filter(([command]) => command === 'close_surface')).toHaveLength(0)
    key('Escape'); await flushPromises()
    expect(desktop.invoke).toHaveBeenCalledWith('close_current_surface')
  })

  it('une confirmation ancienne nefface et neferme pas la capture rouverte', async () => {
    // RG-F1-03/F5-05 — Étant donné une capture validée dont le worker est encore actif.
    const confirmation = deferred<void>()
    desktop.invoke.mockImplementation(async command => command === 'desktop_status' ? status : command === 'task_create' ? confirmation.promise : undefined)
    const app = await open('capture')
    await app.get('#capture').setValue('Ancienne capture')
    await app.get('form').trigger('submit')
    // Quand l'utilisateur ferme puis rouvre et écrit avant l'ancienne confirmation.
    emit('surface-closed', 10)
    emit('surface-opened', { activation: 11, reset: true })
    await flushPromises()
    await app.get('#capture').setValue('Nouvelle capture')
    confirmation.resolve()
    await flushPromises()
    // Alors seule l'ancienne écriture est confirmée, la nouvelle surface garde son brouillon.
    expect(app.get<HTMLInputElement>('#capture').element.value).toBe('Nouvelle capture')
    expect(desktop.invoke.mock.calls.filter(([command]) => command === 'close_surface')).toHaveLength(0)
    expect(app.get<HTMLInputElement>('#capture').element.readOnly).toBe(false)
  })

  it('une erreur ancienne propose de récupérer son brouillon sans écraser le nouveau', async () => {
    const confirmation = deferred<void>()
    desktop.invoke.mockImplementation(async command => command === 'desktop_status' ? status : command === 'task_create' ? confirmation.promise : undefined)
    const app = await open('capture')
    await app.get('#capture').setValue('À récupérer')
    await app.get('form').trigger('submit')
    emit('surface-closed', 10); emit('surface-opened', { activation: 11, reset: true })
    await flushPromises()
    await app.get('#capture').setValue('Nouveau brouillon')
    confirmation.reject(new Error('Disque inaccessible'))
    await flushPromises()
    expect(app.get<HTMLInputElement>('#capture').element.value).toBe('Nouveau brouillon')
    expect(app.text()).toContain('Récupérer une saisie non enregistrée')
    expect(app.get<HTMLButtonElement>('button.secondary').element.disabled).toBe(true)
    await app.get('button.secondary').trigger('click')
    expect(app.get<HTMLInputElement>('#capture').element.value).toBe('Nouveau brouillon')
    await app.get('#capture').setValue('')
    await app.get('button.secondary').trigger('click')
    expect(app.get<HTMLInputElement>('#capture').element.value).toBe('À récupérer')
    expect(app.get('[role=alert]').text()).toContain('Disque inaccessible')
  })

  it('une récupération interdite pendant lenregistrement conserve le brouillon échoué', async () => {
    const first = deferred<void>()
    const second = deferred<void>()
    const app = await open('capture')
    desktop.invoke.mockImplementation(async command => command === 'task_create' ? first.promise : undefined)
    await app.get('#capture').setValue('Capture échouée A')
    await app.get('form').trigger('submit')
    emit('surface-closed', 10); emit('surface-opened', { activation: 11, reset: true }); await flushPromises()
    first.reject(new Error('Échec A')); await flushPromises()
    desktop.invoke.mockImplementation(async command => command === 'task_create' ? second.promise : undefined)
    await app.get('#capture').setValue('Capture valide B')
    await app.get('form').trigger('submit')
    expect(app.get<HTMLButtonElement>('button.secondary').element.disabled).toBe(true)
    await app.get('button.secondary').trigger('click')
    expect(app.get<HTMLInputElement>('#capture').element.value).toBe('Capture valide B')
    second.resolve(); await flushPromises()
    emit('surface-closed', 11); emit('surface-opened', { activation: 12, reset: true }); await flushPromises()
    await app.get('button.secondary').trigger('click')
    expect(app.get<HTMLInputElement>('#capture').element.value).toBe('Capture échouée A')
    expect(app.get('[role=alert]').text()).toContain('Échec A')
  })
})

describe('panneau accessible au clavier', () => {
  it('la croix masque toujours le panneau actuellement affiché', async () => {
    // RG-F0-02/F5-04 — Étant donné le panneau principal prêt à recevoir une action.
    const app = await open('panel')
    // Quand l'utilisateur clique sa croix de fermeture.
    await app.get('[aria-label="Fermer"]').trigger('click')
    // Alors la demande concerne la surface courante, sans dépendre d'une activation périmée.
    expect(desktop.invoke).toHaveBeenCalledWith('close_current_surface')
  })

  it('les flèches sélectionnent puis Espace complète la tâche avec sa source courante', async () => {
    // RG-F2-14/F3-08 — Étant donné deux tâches affichées et le focus dans la liste.
    const app = await open('panel')
    expect(document.activeElement).toBe(app.get('.task-list').element)
    key('ArrowDown'); await flushPromises()
    expect(app.get('[data-position="1"]').classes()).toContain('selected')
    // Quand Espace bascule la sélection.
    key(' '); await flushPromises()
    // Alors l'intention cible la position et génération projetées, pas un indice de liste filtrée.
    expect(desktop.invoke).toHaveBeenCalledWith('task_toggle', { source: { generation: 2, revision: 7 }, position: 1, request: { query: '', tag: null, history: false } })
  })

  it('les champs gardent flèches et espaces, Entrée recherche rend le focus à la liste', async () => {
    const app = await open('panel')
    key('f', { ctrlKey: true }); await flushPromises()
    const search = app.get<HTMLInputElement>('#search')
    expect(document.activeElement).toBe(search.element)
    await search.setValue('support mission')
    const arrow = key('ArrowDown'); key(' '); await flushPromises()
    expect(arrow.defaultPrevented).toBe(false)
    expect(app.get('[data-position="0"]').classes()).toContain('selected')
    expect(desktop.invoke.mock.calls.filter(([command]) => command === 'task_toggle' || command === 'task_reorder')).toHaveLength(0)
    await search.trigger('keydown', { key: 'Enter' }); await flushPromises()
    expect(document.activeElement).toBe(app.get('.task-list').element)
    expect(search.element.value).toBe('support mission')
  })

  it('historique conserve recherche et tag, puis Échap consomme recherche avant historique', async () => {
    const app = await open('panel')
    await app.get('[aria-label="Filtrer par #mission"]').trigger('click')
    key('f', { ctrlKey: true }); await flushPromises()
    await app.get('#search').setValue('support')
    key('h', { ctrlKey: true }); await flushPromises()
    expect(app.get('h1').text()).toBe('Historique')
    expect(desktop.invoke).toHaveBeenLastCalledWith('task_view', { request: { query: 'support', tag: 'mission', history: true } })
    expect(app.find('input[type=checkbox]').exists()).toBe(false)
    key('Escape'); await flushPromises()
    expect(app.get('h1').text()).toBe('Historique')
    expect(app.find('#search').exists()).toBe(false)
    expect(app.text()).toContain('Filtre : #mission')
    key('Escape'); await flushPromises()
    expect(app.get('h1').text()).toBe('Mes tâches')
    expect(desktop.invoke.mock.calls.filter(([command]) => command === 'close_surface')).toHaveLength(0)
  })

  it('une ancienne révision invalide lédition sans effacer le texte de lutilisateur', async () => {
    const app = await open('panel')
    key('Enter'); await flushPromises()
    await app.get('#edit').setValue('Brouillon important /p lundi')
    desktop.invoke.mockImplementation(async command => command === 'task_view' ? taskView({ version: 22, source: { generation: 3, revision: 7 }, rows: [row(0, 'Autre source')] }) : undefined)
    emit('task-changed', null); await flushPromises()
    expect(app.get<HTMLInputElement>('#edit').element.value).toBe('Brouillon important /p lundi')
    expect(app.get('[role=alert]').text()).toContain('La liste a changé')
    expect(app.get<HTMLButtonElement>('.composer .primary').element.disabled).toBe(true)
    await app.get('form.composer').trigger('submit'); await flushPromises()
    expect(desktop.invoke.mock.calls.filter(([command]) => command === 'task_update')).toHaveLength(0)
    key('Escape'); await flushPromises()
    expect(app.find('#edit').exists()).toBe(false)
  })

  it('une réponse de projection retardée ne remplace pas la dernière vue et son état busy', async () => {
    const app = await open('panel')
    const old = deferred<ReturnType<typeof taskView>>()
    const latest = deferred<ReturnType<typeof taskView>>()
    desktop.invoke.mockImplementationOnce(() => old.promise).mockImplementationOnce(() => latest.promise)
    emit('task-changed', null); emit('task-changed', null)
    latest.resolve(taskView({ version: 23, rows: [row(0, 'Vue récente')] }))
    await flushPromises()
    old.resolve(taskView({ version: 22, busy: true, rows: [row(0, 'Vue ancienne')] }))
    await flushPromises()
    expect(app.text()).toContain('Vue récente')
    expect(app.text()).not.toContain('Vue ancienne')
    expect(app.get<HTMLInputElement>('input[type=checkbox]').element.disabled).toBe(false)
    desktop.invoke.mockImplementationOnce(async () => taskView({ version: 20, busy: true, rows: [row(0, 'Version obsolète')] }))
    emit('task-changed', null); await flushPromises()
    expect(app.text()).not.toContain('Version obsolète')
  })

  it('un déplacement suit la tâche permutée à sa position confirmée', async () => {
    const app = await open('panel')
    desktop.invoke.mockImplementation(async command => command === 'task_reorder' ? 1 : command === 'task_view' ? taskView({ version: 23, source: { generation: 2, revision: 8 }, rows: [row(0, 'Seconde'), row(1, 'Première')] }) : undefined)
    key('ArrowDown', { ctrlKey: true }); await flushPromises()
    expect(desktop.invoke).toHaveBeenCalledWith('task_reorder', { source: { generation: 2, revision: 7 }, position: 0, down: true, request: { query: '', tag: null, history: false } })
    expect(app.get('[data-position="1"]').classes()).toContain('selected')
    expect(app.get('[data-position="1"]').text()).toContain('Première')
  })

  it('redonner le focus au panneau conserve le focus de lédition et de la recherche', async () => {
    // RG-F1-02/F2-05/14 — Étant donné une édition dans un panneau déjà visible.
    const app = await open('panel')
    key('Enter'); await flushPromises()
    await app.get('#edit').setValue('Brouillon gardé')
    // Quand le tray redonne simplement le focus, sans nouvelle activation.
    emit('surface-opened', { activation: 10, reset: false }); await flushPromises()
    // Alors les touches continuent d'agir sur le champ, pas sur la liste sous-jacente.
    expect(document.activeElement).toBe(app.get('#edit').element)
    expect(app.get<HTMLInputElement>('#edit').element.value).toBe('Brouillon gardé')
    key('Escape'); key('f', { ctrlKey: true }); await flushPromises()
    await app.get('#search').setValue('mission')
    emit('surface-opened', { activation: 10, reset: false }); await flushPromises()
    expect(document.activeElement).toBe(app.get('#search').element)
  })

  it('un ajout intégré exclut lédition simultanée dune autre tâche', async () => {
    const app = await open('panel')
    await app.get('.add-task').trigger('click')
    await app.get('#capture').setValue('Nouvelle tâche')
    await app.get('[data-position="0"] .task-title').trigger('click')
    expect(app.find('#edit').exists()).toBe(false)
    expect(app.get<HTMLInputElement>('#capture').element.value).toBe('Nouvelle tâche')
    emit('surface-opened', { activation: 10, reset: false }); await flushPromises()
    expect(document.activeElement).toBe(app.get('#capture').element)
  })

  it('lerreur dune ancienne édition ne pollue pas un nouvel ajout en cours', async () => {
    const edit = deferred<void>()
    const create = deferred<void>()
    const app = await open('panel')
    desktop.invoke.mockImplementation(async command => command === 'task_update' ? edit.promise : command === 'task_create' ? create.promise : command === 'task_view' ? taskView() : undefined)
    key('Enter'); await flushPromises()
    await app.get('#edit').setValue('Ancienne édition')
    await app.get('form.composer').trigger('submit')
    emit('surface-closed', 10); emit('surface-opened', { activation: 11, reset: true }); await flushPromises()
    await app.get('.add-task').trigger('click')
    await app.get('#capture').setValue('Nouvel ajout')
    await app.get('form.composer').trigger('submit')
    edit.reject(new Error('Ancien conflit'))
    await flushPromises()
    expect(app.find('[role=alert]').exists()).toBe(false)
    expect(app.get<HTMLInputElement>('#capture').element.readOnly).toBe(true)
    expect(app.get<HTMLInputElement>('#capture').element.value).toBe('Nouvel ajout')
    create.resolve(); await flushPromises()
  })

  it.each(['task_toggle', 'task_reorder'])('la réponse tardive de %s préserve le nouvel ajout et sa sélection', async command => {
    const old = deferred<unknown>()
    const create = deferred<void>()
    const app = await open('panel')
    desktop.invoke.mockImplementation(async requested => requested === command ? old.promise : requested === 'task_create' ? create.promise : requested === 'task_view' ? taskView() : undefined)
    if (command === 'task_toggle') key(' ')
    else key('ArrowDown', { ctrlKey: true })
    await flushPromises()
    emit('surface-closed', 10); emit('surface-opened', { activation: 11, reset: true }); await flushPromises()
    await app.get('.add-task').trigger('click')
    await app.get('#capture').setValue('Nouvelle activation')
    await app.get('form.composer').trigger('submit')
    if (command === 'task_toggle') old.reject(new Error('Ancienne erreur'))
    else old.resolve(1)
    await flushPromises()
    expect(app.find('[role=alert]').exists()).toBe(false)
    expect(app.get<HTMLInputElement>('#capture').element.readOnly).toBe(true)
    expect(app.get('[data-position="0"]').classes()).toContain('selected')
    create.resolve(); await flushPromises()
  })
})

describe('paramètres', () => {
  it('présente les états actifs et conserve la combinaison refusée pour correction', async () => {
    const app = await open('settings')
    expect(app.get('#capture-shortcut').element).toHaveProperty('value', 'Ctrl+Shift+Espace')
    expect(app.get('#panel-shortcut').element).toHaveProperty('value', 'Ctrl+Shift+T')
    expect(app.text()).toContain('Inactif')
    desktop.invoke.mockImplementation(async command => { if (command === 'settings_shortcut') throw new Error('Raccourcis identiques') })
    await app.get('#capture-shortcut').setValue('Ctrl+Shift+T')
    await app.get('#capture-shortcut').element.closest('form')!.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }))
    await flushPromises()
    expect(desktop.invoke).toHaveBeenCalledWith('settings_shortcut', { capture: true, value: 'Ctrl+Shift+T' })
    expect(app.get('[role=alert]').text()).toContain('Raccourcis identiques')
    expect(app.get<HTMLInputElement>('#capture-shortcut').element.value).toBe('Ctrl+Shift+T')
    expect(app.get<HTMLInputElement>('#panel-shortcut').element.value).toBe('Ctrl+Shift+T')
    expect(app.get<HTMLButtonElement>('#capture-shortcut + button').element.disabled).toBe(false)
  })

  it('un diagnostic post-enregistrement en erreur libère le formulaire pour réessayer', async () => {
    const app = await open('settings')
    desktop.invoke.mockImplementation(async command => {
      if (command === 'desktop_status') throw new Error('Diagnostic indisponible')
    })
    await app.get('#capture-shortcut').setValue('Alt+Shift+C')
    app.get('#capture-shortcut').element.closest('form')!.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }))
    await flushPromises()
    expect(app.get('[role=alert]').text()).toContain('Diagnostic indisponible')
    expect(app.get<HTMLButtonElement>('#capture-shortcut + button').element.disabled).toBe(false)
    desktop.invoke.mockImplementation(async command => command === 'desktop_status' ? status : undefined)
    app.get('#capture-shortcut').element.closest('form')!.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }))
    await flushPromises()
    expect(app.get('[role=status]').text()).toContain('enregistré')
  })

  it('un dialogue de fichier revenu après réouverture ne change pas la nouvelle activation', async () => {
    const chosen = deferred<string>()
    const app = await open('settings')
    desktop.invoke.mockImplementation(async command => command === 'settings_choose' ? chosen.promise : command === 'desktop_status' ? status : undefined)
    const choose = app.findAll('button').find(button => button.text() === 'Choisir un fichier…')!
    await choose.trigger('click')
    emit('surface-closed', 10); emit('surface-opened', { activation: 11, reset: true, settings: true }); await flushPromises()
    chosen.resolve('/tmp/ancienne-selection.md'); await flushPromises()
    expect(app.get<HTMLInputElement>('#file-path').element.value).toBe(status.tasksFile)
    expect(desktop.invoke.mock.calls.filter(([command]) => command === 'settings_source')).toHaveLength(0)
    expect(choose.element.disabled).toBe(false)
  })

  it('une erreur de réglage ancienne ne remplace pas les messages de la nouvelle activation', async () => {
    const saved = deferred<void>()
    const app = await open('settings')
    desktop.invoke.mockImplementation(async command => command === 'settings_shortcut' ? saved.promise : command === 'desktop_status' ? status : undefined)
    app.get('#capture-shortcut').element.closest('form')!.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }))
    await flushPromises()
    emit('surface-closed', 10); emit('surface-opened', { activation: 11, reset: true, settings: true }); await flushPromises()
    saved.reject(new Error('Ancienne erreur de paramètres')); await flushPromises()
    expect(app.find('[role=alert]').exists()).toBe(false)
    expect(app.get<HTMLButtonElement>('#capture-shortcut + button').element.disabled).toBe(false)
  })
})
