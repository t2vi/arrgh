# ── Fetch verified plugin bundles from their own repos' releases (spec 031 phase D) ───────────
# Every plugin now lives in its own t2vi/arrgh-plugin-<id> repo; the image build fetches each
# bundled catalog entry's release asset and verifies its sha256 + its own info.version, failing
# the build on any mismatch (FR-016) — see plugin-index/index.json and
# scripts/fetch-plugin-bundles.mjs. Force amd64: the fetched files are platform-agnostic JS, and
# Node 22 crashes under QEMU arm64 emulation for the rest of this file's build stages.

FROM --platform=linux/amd64 node:22-alpine AS bundles
WORKDIR /build
COPY plugin-index/index.json          ./plugin-index/
COPY scripts/fetch-plugin-bundles.mjs ./scripts/
RUN node scripts/fetch-plugin-bundles.mjs --out /build/bundles

# ── Workspace deps (shared build-time install) ────────────────────────────────

FROM --platform=linux/amd64 node:22-alpine AS deps
WORKDIR /build
COPY package.json package-lock.json ./
COPY plugin-host/package.json       ./plugin-host/
RUN npm ci

# ── Build plugin-host ─────────────────────────────────────────────────────────

FROM --platform=linux/amd64 deps AS host-build
COPY plugin-host/tsconfig.json ./plugin-host/
COPY plugin-host/src           ./plugin-host/src
WORKDIR /build/plugin-host
RUN npm run build

# ── Prod deps (standalone install for the runtime image) ─────────────────────
# Run on amd64 to avoid QEMU crash. plugin-host deps are pure JS — amd64
# node_modules work on arm64 at runtime.

FROM --platform=linux/amd64 node:22-alpine AS prod-deps
WORKDIR /app
COPY plugin-host/package.json ./
# git: npm needs it to resolve the arrgh-plugin-sdk git devDependency before --omit=dev drops it
# (alpine's node image doesn't ship git); the SDK itself is dev/test-only, never in node_modules here.
RUN apk add --no-cache git && npm install --omit=dev --ignore-scripts

# ── Final image ───────────────────────────────────────────────────────────────

FROM node:22-alpine
WORKDIR /app
COPY --from=prod-deps /app/node_modules ./node_modules
COPY --from=host-build /build/plugin-host/dist ./dist
COPY --from=bundles /build/bundles ./bundles
ENV PORT=4000 LANGUAGES=en CLOAKBROWSER_WS_URL=
EXPOSE 4000
CMD ["node", "dist/index.js"]
