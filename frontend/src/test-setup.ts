// Setup per a tests Vitest amb jsdom
// Utilitza fake-indexeddb per una implementació real d'IndexedDB

import { IDBFactory } from 'fake-indexeddb'

// Configurar global indexedDB amb fake-indexeddb
;(globalThis as any).indexedDB = new IDBFactory()