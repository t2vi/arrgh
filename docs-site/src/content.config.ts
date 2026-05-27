import { defineCollection, z } from 'astro:content'
import { glob } from 'astro/loaders'

const deploy = defineCollection({
  loader: glob({ pattern: '**/*.md', base: '../docs/deploy' }),
  schema: z.object({}).passthrough(),
})

const releases = defineCollection({
  loader: glob({ pattern: '**/*.md', base: '../docs/releases' }),
  schema: z.object({}).passthrough(),
})

export const collections = { deploy, releases }
