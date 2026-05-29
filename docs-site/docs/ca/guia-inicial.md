# Guia inicial

Aquest portal serveix per publicar la documentacio de ChillGroup com a web estatica a GitHub Pages.

## Que hi trobaras

- Una entrada rapida al projecte.
- Enllacos a tota l'especificacio tecnica existent.
- Una estructura bilingue per ampliar contingut en catala i angles.

## Mapa del projecte

- `definitions/`: font original de la documentacio tecnica.
- `docs-site/`: projecte VitePress per generar la web.
- `frontend/`: client React de l'aplicacio.
- `server/`: backend Rust amb Axum i SQLx.

## Flux recomanat

1. Actualitza la documentacio tecnica a `definitions/`.
2. Executa el build del portal de docs.
3. GitHub Pages publica la versio estatica generada.

## Per on comencar

- Si vols entendre el producte, comenca per [Overview](/ca/reference/OVERVIEW).
- Si vols revisar l'arquitectura, ves a [Architecture](/ca/reference/ARCHITECTURE).
- Si vols desplegar o contribuir, mira [Development](/ca/reference/DEVELOPMENT).

## Notes sobre idiomes

La referencia completa neix ara mateix de la documentacio existent en catala. La seccio anglesa del portal ofereix una entrada equivalent i es pot anar ampliant per traduir les peces mes importants de forma incremental.