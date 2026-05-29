import { cp, mkdir, readdir, readFile, rm, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const projectRoot = path.resolve(__dirname, '..', '..');
const sourceDir = path.join(projectRoot, 'definitions');
const targetDir = path.join(projectRoot, 'docs-site', 'docs', 'ca', 'reference');

async function main() {
  await rm(targetDir, { recursive: true, force: true });
  await mkdir(targetDir, { recursive: true });

  const entries = await readdir(sourceDir, { withFileTypes: true });

  for (const entry of entries) {
    if (!entry.isFile() || !entry.name.endsWith('.md')) {
      continue;
    }

    const sourcePath = path.join(sourceDir, entry.name);
    const targetName = entry.name === 'README.md' ? 'index.md' : entry.name;
    const targetPath = path.join(targetDir, targetName);

    if (entry.name === 'README.md') {
      const content = await readFile(sourcePath, 'utf8');
      const normalizedContent = content.replace(/\]\(([^)]+)\.md\)/g, ']($1.md)');
      await writeFile(targetPath, normalizedContent, 'utf8');
      continue;
    }

    await cp(sourcePath, targetPath);
  }
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});