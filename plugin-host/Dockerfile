# ── Workspace deps (shared build-time install) ────────────────────────────────

FROM node:22-alpine AS deps
WORKDIR /build
COPY package.json package-lock.json ./
COPY plugins/mangadex/package.json   ./plugins/mangadex/
COPY plugins/toonily/package.json    ./plugins/toonily/
COPY plugins/novelfull/package.json  ./plugins/novelfull/
COPY plugins/mangapill/package.json  ./plugins/mangapill/
COPY plugins/nhentai/package.json    ./plugins/nhentai/
COPY plugins/mangafire/package.json  ./plugins/mangafire/
COPY plugins/asurascans/package.json ./plugins/asurascans/
COPY plugins/manhuafast/package.json ./plugins/manhuafast/
COPY plugins/wuxiaworld/package.json ./plugins/wuxiaworld/
COPY plugins/boxnovel/package.json       ./plugins/boxnovel/
COPY plugins/novelupdates/package.json   ./plugins/novelupdates/
COPY plugins/manga18fx/package.json      ./plugins/manga18fx/
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
COPY --from=bundles /build/plugins/mangadex/bundles/mangadex.js     ./bundles/
COPY --from=bundles /build/plugins/toonily/bundles/toonily.js       ./bundles/
COPY --from=bundles /build/plugins/novelfull/bundles/novelfull.js   ./bundles/
COPY --from=bundles /build/plugins/mangapill/bundles/mangapill.js   ./bundles/
COPY --from=bundles /build/plugins/nhentai/bundles/nhentai.js       ./bundles/
COPY --from=bundles /build/plugins/mangafire/bundles/mangafire.js   ./bundles/
COPY --from=bundles /build/plugins/asurascans/bundles/asurascans.js ./bundles/
COPY --from=bundles /build/plugins/manhuafast/bundles/manhuafast.js ./bundles/
COPY --from=bundles /build/plugins/wuxiaworld/bundles/wuxiaworld.js ./bundles/
COPY --from=bundles /build/plugins/boxnovel/bundles/boxnovel.js         ./bundles/
COPY --from=bundles /build/plugins/novelupdates/bundles/novelupdates.js ./bundles/
COPY --from=bundles /build/plugins/manga18fx/bundles/manga18fx.js       ./bundles/
ENV PORT=4000 LANGUAGES=en CLOAKBROWSER_WS_URL=
EXPOSE 4000
CMD ["node", "dist/index.js"]
