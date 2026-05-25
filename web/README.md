# arrgh — web

React + TypeScript SPA. Proxies `/api/*` to the server during development.

## Dev setup

```bash
cd web
npm install
npm run dev
```

UI at `http://localhost:5173`. Requires the server running at `localhost:3000` (Vite proxy handles `/api` forwarding).

## Build

```bash
npm run build
# output at web/dist/ — serve with nginx or any static file server
```

## Tests

```bash
npm test
```

## Project structure

```
src/
├── lib/
│   ├── api.ts        # API client + all shared domain types
│   └── utils.ts      # cn() utility
├── features/         # Page-level features (library, discover, reader…)
│   └── <name>/
│       ├── index.tsx       # Thin layout shell
│       ├── hooks/          # All state logic (useEffect/useState — no TanStack)
│       └── components/     # Feature-local components
├── components/       # Shared UI components + shadcn primitives
└── hooks/            # Cross-feature hooks
```

Types live in `src/lib/api.ts` (shared) or inline in the feature file. `src/types.ts` is deprecated — migrate on touch.
