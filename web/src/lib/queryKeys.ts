export const queryKeys = {
  manga: {
    all: () => ['manga'] as const,
    list: (page: number, search: string) => ['manga', page, search] as const,
    detail: (id: string) => ['manga', id] as const,
  },
  chapters: {
    list: (mangaId: string) => ['chapters', mangaId] as const,
  },
  progress: {
    manga: (mangaId: string) => ['progress-manga', mangaId] as const,
    chapter: (chapterId: string) => ['progress', chapterId] as const,
  },
  queue: {
    all: () => ['queue'] as const,
    manga: (mangaId: string) => ['queue-manga', mangaId] as const,
  },
  trending: () => ['trending'] as const,
  newReleases: () => ['new-releases'] as const,
  continueReading: () => ['continue-reading'] as const,
  discoverDetail: (source: string, sourceId: string) =>
    ['discover-detail', source, sourceId] as const,
}
