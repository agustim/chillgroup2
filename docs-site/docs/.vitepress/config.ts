import { defineConfig } from 'vitepress';

const repositoryName = process.env.GITHUB_REPOSITORY?.split('/')[1];
const base = repositoryName ? `/${repositoryName}/` : '/';

export default defineConfig({
  base,
  cleanUrls: true,
  lang: 'ca',
  title: 'ChillGroup Docs',
  description: 'Documentacio tecnica del projecte ChillGroup en Catala i angles.',
  lastUpdated: true,
  head: [
    ['meta', { name: 'theme-color', content: '#0f766e' }],
    ['meta', { property: 'og:title', content: 'ChillGroup Docs' }],
    ['meta', { property: 'og:description', content: 'Portal de documentacio tecnica de ChillGroup.' }]
  ],
  themeConfig: {
    logo: '/logo.svg',
    nav: [
      { text: 'Catala', link: '/ca/' },
      { text: 'English', link: '/en/' },
      { text: 'Referencia', link: '/ca/reference/' }
    ],
    socialLinks: [
      { icon: 'github', link: 'https://github.com/' }
    ],
    search: {
      provider: 'local'
    },
    footer: {
      message: 'ChillGroup documentation portal',
      copyright: 'Projecte QuantumTeam'
    },
    sidebar: {
      '/ca/': [
        {
          text: 'Guia',
          items: [
            { text: 'Inici', link: '/ca/' },
            { text: 'Guia inicial', link: '/ca/guia-inicial' },
            { text: 'Deploy amb Docker', link: '/ca/deploy-docker' },
            { text: 'Com contribuir', link: '/ca/contribuir' },
            { text: 'Referencia tecnica', link: '/ca/reference/' }
          ]
        },
        {
          text: 'Especificacio',
          items: [
            { text: 'Overview', link: '/ca/reference/OVERVIEW' },
            { text: 'Architecture', link: '/ca/reference/ARCHITECTURE' },
            { text: 'Cryptography', link: '/ca/reference/CRYPTOGRAPHY' },
            { text: 'Database', link: '/ca/reference/DATABASE' },
            { text: 'Development', link: '/ca/reference/DEVELOPMENT' },
            { text: 'Frontend', link: '/ca/reference/FRONTEND' },
            { text: 'Testing', link: '/ca/reference/TESTING' },
            { text: 'API', link: '/ca/reference/API' },
            { text: 'DM', link: '/ca/reference/DM' },
            { text: 'Socket', link: '/ca/reference/SOCKET' },
            { text: 'Errors', link: '/ca/reference/ERRORS' }
          ]
        }
      ],
      '/en/': [
        {
          text: 'Guide',
          items: [
            { text: 'Home', link: '/en/' },
            { text: 'Getting started', link: '/en/getting-started' },
            { text: 'Docker deployment', link: '/en/docker-deployment' },
            { text: 'Contributing', link: '/en/contributing' },
            { text: 'Reference', link: '/en/reference/' }
          ]
        },
        {
          text: 'Reference (English)',
          items: [
            { text: 'Overview', link: '/en/reference/OVERVIEW' },
            { text: 'Cryptography', link: '/en/reference/CRYPTOGRAPHY' },
            { text: 'Development', link: '/en/reference/DEVELOPMENT' }
          ]
        },
        {
          text: 'Reference (Catalan)',
          items: [
            { text: 'Architecture', link: '/ca/reference/ARCHITECTURE' },
            { text: 'Database', link: '/ca/reference/DATABASE' },
            { text: 'Frontend', link: '/ca/reference/FRONTEND' },
            { text: 'Testing', link: '/ca/reference/TESTING' },
            { text: 'API', link: '/ca/reference/API' },
            { text: 'DM', link: '/ca/reference/DM' },
            { text: 'Socket', link: '/ca/reference/SOCKET' },
            { text: 'Errors', link: '/ca/reference/ERRORS' }
          ]
        }
      ]
    }
  },
  locales: {
    root: {
      label: 'Catala',
      lang: 'ca',
      link: '/ca/',
      themeConfig: {
        nav: [
          { text: 'Catala', link: '/ca/' },
          { text: 'English', link: '/en/' },
          { text: 'Referencia', link: '/ca/reference/' }
        ]
      }
    },
    en: {
      label: 'English',
      lang: 'en',
      link: '/en/',
      themeConfig: {
        nav: [
          { text: 'Catalan', link: '/ca/' },
          { text: 'English', link: '/en/' },
          { text: 'Reference', link: '/en/reference/' }
        ]
      }
    }
  },
  srcExclude: ['README.md']
});