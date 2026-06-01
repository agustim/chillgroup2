// Setup per a tests Vitest amb jsdom
// Utilitza fake-indexeddb per una implementació real d'IndexedDB

import { IDBFactory } from 'fake-indexeddb'

// Configurar global indexedDB amb fake-indexeddb
;(globalThis as any).indexedDB = new IDBFactory()

// Alguns entorns de Vitest poden no exposar localStorage.
if (!(globalThis as any).localStorage) {
	const storage = new Map<string, string>()
	;(globalThis as any).localStorage = {
		getItem: (key: string) => (storage.has(key) ? storage.get(key)! : null),
		setItem: (key: string, value: string) => {
			storage.set(key, String(value))
		},
		removeItem: (key: string) => {
			storage.delete(key)
		},
		clear: () => {
			storage.clear()
		},
	}
}