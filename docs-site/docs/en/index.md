---
layout: home

hero:
  name: ChillGroup
  text: Secure, quantum-resistant messaging
  tagline: Build your own communication servers — chat, voice and video with real E2EE and post-quantum cryptography. Open source and self-hostable.
  actions:
    - theme: brand
      text: Get started
      link: /en/getting-started
    - theme: alt
      text: Technical reference
      link: /en/reference/
    - theme: alt
      text: Contribute
      link: /en/contributing

features:
  - icon: 🔐
    title: True Zero-Knowledge
    details: On asymmetric channels, the server never sees your keys or your messages. Channel keys are encrypted with Kyber-1024 (ML-KEM-1024, NIST Level 5) — only your device can decrypt them.
  - icon: ⚛️
    title: Post-quantum by design
    details: Unlike Discord, Element or Telegram, ChillGroup uses quantum-resistant cryptography for all E2EE channels, protecting your conversations against future quantum computer attacks.
  - icon: 🏠
    title: Self-hosted
    details: A single `docker compose up` is all you need to run your own instance. No external accounts, no telemetry, no lock-in.
  - icon: 🎙️
    title: E2EE voice and video
    details: Audio and video calls via LiveKit with end-to-end encryption using independent session keys, separate from the chat key system.
  - icon: 🛡️
    title: Three security levels
    details: Open channel, symmetric encryption (AES-256-GCM), or full asymmetric E2EE. Each channel picks its own level — security is not all-or-nothing.
  - icon: ⚙️
    title: Rust + React
    details: Backend in Rust (Axum + SQLx) for maximum performance and memory safety. Frontend in React + TypeScript. Fully typed, fully tested.
---