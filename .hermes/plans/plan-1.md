# Plan: Arreglar tests + commit a dev

## 1. Fix tests que fallen (2-3)

**Tests que fallen actualment (2 de 95):**
- `api.test.ts` → authRegister: mock fetch espera URL `http://localhost:8080/api/auth/register` però la crida va a `/api/auth/register` (sense prefix)
- `api.test.ts` → authLogin: mateix problema

**Solució:** Fixar els mocks de `api.test.ts` perquè coincideixin amb la URL real (sense `localhost:8080` prefix)

**Condicions d'error:**
- Si no es poden arreglar els tests sense trencar la lògica → marcar com a tests preexistents i saltar
- Si el test falla per un altre motiu → investigar abans de tocar

## 2. Verificar Rust compila amb SQLite

**Pas:**
```bash
cd server && cargo check 2>&1
```

**Condicions d'error:**
- Si no compila → arreglar els errors (dependències, tipus, etc.)
- Si falla per falta de dependències → afegir-les al Cargo.toml

## 3. Commit a dev branch

**Pas:**
1. Crear branch `dev` local si no existeix
2. Stage tots els fitxers canviats
3. Commit amb missatge descriptiu
4. Push a `origin/dev`

**Condicions d'error:**
- Si `origin/dev` no existeix → crear-lo
- Si hi ha conflictes → resoldre-los manualment
- Si el push falla per permisos → avisar l'usuari

## 4. Verificació final

**Pas:**
- `git log --oneline -5` per confirmar el commit
- `git push origin dev` per confirmar que puja correctament

**Condicions d'error:**
- Si el commit no es pot fer → verificar què ha canviat amb `git diff`
- Si el push no funciona → verificar les credencials de Git
