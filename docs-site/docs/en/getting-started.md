# Getting started

This documentation site is intended to expose ChillGroup's project docs as a static website on GitHub Pages.

## What is included

- An English landing experience for the project.
- Direct access to the full technical specification currently authored in Catalan.
- A structure that can grow into complete bilingual documentation over time.

## Project map

- `definitions/`: original technical source documents.
- `docs-site/`: VitePress app that turns those docs into a website.
- `frontend/`: React client.
- `server/`: Rust backend.

## Suggested workflow

1. Update technical specs inside `definitions/`.
2. Build the docs site.
3. Publish the generated static output to GitHub Pages.

## Project deployment

The project publishes a pre-built Docker image on every release. To deploy, run the interactive wizard that generates a `docker-compose.yml` and `.env.compose` tailored to your infrastructure:

```bash
curl -fsSL https://raw.githubusercontent.com/agustim/chillgroup2/main/setup-deploy.sh -o setup-deploy.sh
bash setup-deploy.sh
```

Full guide: [Docker deployment](/en/docker-deployment).

## Recommended reading order

- Start with [the reference overview](/ca/reference/OVERVIEW) to understand the product goals.
- Continue with [architecture](/ca/reference/ARCHITECTURE) for the backend and frontend split.
- Use [development](/ca/reference/DEVELOPMENT) when you need build and contribution details.

## Translation approach

The English section currently focuses on orientation and navigation. The detailed specification remains synced from the Catalan source documents, which keeps the website accurate while translations are expanded incrementally.