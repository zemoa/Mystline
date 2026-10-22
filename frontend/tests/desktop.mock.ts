import { vi } from 'vitest'

const desktop = vi.hoisted(() => ({
  invoke: vi.fn(),
  listeners: new Map<string, (event: { payload: unknown }) => void>(),
  removed: vi.fn(),
}))
export { desktop }

vi.mock('@tauri-apps/api/core', () => ({ invoke: desktop.invoke }))
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(async (name: string, listener: (event: { payload: unknown }) => void) => {
    desktop.listeners.set(name, listener)
    return () => { desktop.listeners.delete(name); desktop.removed(name) }
  }),
}))

export function emit(name: string, payload: unknown) {
  const listener = desktop.listeners.get(name)
  if (!listener) throw new Error(`L'événement ${name} n'est pas écouté`)
  listener({ payload })
}

export function deferred<T>() {
  let resolve!: (value: T) => void
  let reject!: (error: Error) => void
  const promise = new Promise<T>((accept, refuse) => { resolve = accept; reject = refuse })
  return { promise, resolve, reject }
}

export const status = {
  version: 21,
  tasksFile: '/tmp/mystline-tests/tasks.md',
  taskCount: 2,
  revision: 7,
  today: '2026-10-09',
  autostart: true,
  captureShortcut: 'Ctrl+Shift+Espace',
  panelShortcut: 'Ctrl+Shift+T',
  captureActive: true,
  panelActive: false,
  repositoryError: null,
}

export function row(position: number, title: string, patch: Record<string, unknown> = {}) {
  return {
    position, title, section: 'current', tags: ['mission'], planned: null, deadline: null,
    deadlineTone: null, completed: false, completedDate: null, editDraft: `${title} #mission`,
    ...patch,
  }
}

export function taskView(patch: Record<string, unknown> = {}) {
  return {
    version: 21, source: { generation: 2, revision: 7 }, today: '2026-10-09',
    nextWorkday: '2026-10-12', busy: false, rows: [row(0, 'Première'), row(1, 'Seconde')],
    ...patch,
  }
}
