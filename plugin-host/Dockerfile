# ── Workspace deps (shared build-time install) ────────────────────────────────

FROM node:22-alpine AS deps
WORKDIR /build
COPY package.json package-lock.json ./
COPY plugins/mangadex/package.json  ./plugins/mangadex/
COPY plugins/toonily/package.json   ./plugins/toonily/
COPY plugins/comick/package.json    ./plugins/comick/
COPY plugins/novelfull/package.json ./plugins/novelfull/
COPY plugins/mangapill/package.json ./plugins/mangapill/
COPY plugins/royalroad/package.json ./plugins/royalroad/
COPY plugin-host/package.json       ./plugin-host/
RUN npm ci

# ── Build all plugin bundles ──────────────────────────────────────────────────

FROM deps AS bundles
COPY plugins ./plugins
RUN npm run build:plugins

# ── Build plugin-host ─────────────────────────────────────────────────────────

FROM deps AS host-build
COPY plugin-host/tsconfig.json ./plugin-host/
COPY plugin-host/src           ./plugin-host/src
WORKDIR /build/plugin-host
RUN npm run build

# ── Final image ───────────────────────────────────────────────────────────────

FROM node:22-alpine
WORKDIR /app
COPY plugin-host/package.json ./
RUN npm install --omit=dev --ignore-scripts
COPY --from=host-build /build/plugin-host/dist ./dist
RUN mkdir -p bundles
COPY --from=bundles /build/plugins/mangadex/bundles/mangadex.js   ./bundles/
COPY --from=bundles /build/plugins/toonily/bundles/toonily.js     ./bundles/
COPY --from=bundles /build/plugins/comick/bundles/comick.js       ./bundles/
COPY --from=bundles /build/plugins/novelfull/bundles/novelfull.js ./bundles/
COPY --from=bundles /build/plugins/mangapill/bundles/mangapill.js ./bundles/
COPY --from=bundles /build/plugins/royalroad/bundles/royalroad.js ./bundles/
ENV PORT=4000 LANGUAGES=en CLOAKBROWSER_WS_URL=
EXPOSE 4000
CMD ["node", "dist/index.js"]
