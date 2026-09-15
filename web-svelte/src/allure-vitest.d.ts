// allure-vitest v3's package.json "exports" map has no "." entry, so its
// ambient `allure` global type (dist/index.d.ts) can't be resolved via a
// normal import under bundler moduleResolution. Mirrors that file's
// declaration by hand instead — see test-setup.ts for the runtime setup.
declare global {
  var allure: {
    label: (name: string, value: string) => Promise<void>
    link: (type: string, url: string, name?: string) => Promise<void>
    parameter: (
      name: string,
      value: string,
      options?: { excluded?: boolean; mode?: 'hidden' | 'masked' | 'default' },
    ) => Promise<void>
    description: (markdown: string) => Promise<void>
    descriptionHtml: (html: string) => Promise<void>
    testCaseId: (id: string) => Promise<void>
    historyId: (id: string) => Promise<void>
    allureId: (id: string) => Promise<void>
    displayName: (name: string) => Promise<void>
    attachment: (name: string, content: Buffer | string, type: string) => Promise<void>
    issue: (name: string, url: string) => Promise<void>
    tms: (name: string, url: string) => Promise<void>
    epic: (name: string) => Promise<void>
    feature: (name: string) => Promise<void>
    story: (name: string) => Promise<void>
    suite: (name: string) => Promise<void>
    parentSuite: (name: string) => Promise<void>
    subSuite: (name: string) => Promise<void>
    owner: (name: string) => Promise<void>
    severity: (name: string) => Promise<void>
    layer: (name: string) => Promise<void>
    tag: (name: string) => Promise<void>
    step: (name: string, body: () => Promise<void>) => Promise<void>
  }
}

export {}
