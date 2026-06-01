---
layout: home

hero:
  name: ChillGroup
  text: Missatgeria segura i quantum-resistent
  tagline: Crea els teus propis servidors de comunicació — xat, veu i vídeo amb xifrat E2EE real i resistència als ordinadors quàntics. Open source i auto-allotjable.
  actions:
    - theme: brand
      text: Guia d'inici
      link: /ca/guia-inicial
    - theme: alt
      text: Referència tècnica
      link: /ca/reference/
    - theme: alt
      text: Com contribuir
      link: /ca/contribuir

features:
  - icon: 🔐
    title: Zero-Knowledge real
    details: Als canals asimètrics, el servidor mai veu les teves claus ni els teus missatges. Xifrat Kyber-1024 (ML-KEM-1024, NIST Level 5) per bescanvi de claus.
  - icon: ⚛️
    title: Post-quàntic des del disseny
    details: A diferència de Discord, Element o Telegram, ChillGroup usa criptografia resistent a ordinadors quàntics en tots els canals amb E2EE.
  - icon: 🏠
    title: Auto-allotjat
    details: Un `docker compose up` és tot el que necessites per tenir la teva pròpia instància. Sense comptes externs ni telemetria.
  - icon: 🎙️
    title: Veu i vídeo E2EE
    details: Trucades d'àudio i vídeo via LiveKit amb xifrat extrem a extrem independent del xat.
  - icon: 🛡️
    title: Tres nivells de seguretat
    details: Canal obert, xifrat simètric (AES-256) o xifrat asimètric complet (E2EE). Cada canal tria el seu nivell.
  - icon: ⚙️
    title: Rust + React
    details: Backend en Rust (Axum + SQLx) per màxima eficiència i seguretat de memòria. Frontend React + TypeScript. Tot tipat, tot testejat.
---