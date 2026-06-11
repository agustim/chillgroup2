// Setup per a tests Vitest amb jsdom
// Utilitza fake-indexeddb per una implementació real d'IndexedDB

import { IDBFactory } from 'fake-indexeddb'
import i18n from './i18n'

// Configurar global indexedDB amb fake-indexeddb
;(globalThis as any).indexedDB = new IDBFactory()

// Idioma fix als tests perquè les asercions de text siguin deterministes
// (sense detecció per navigator/localStorage).
void i18n.changeLanguage('ca')

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