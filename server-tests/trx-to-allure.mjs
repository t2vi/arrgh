#!/usr/bin/env node
// Converts .NET TRX test result files → Allure JSON result files (one per test case).
// Sets layer=unit or layer=integration so results appear in the correct tier.
//
// Usage: node trx-to-allure.mjs <results.trx> <allure-results-dir> <layer>
//   layer: "unit" | "integration"

import { readFileSync, writeFileSync, mkdirSync } from 'fs'
import { randomUUID } from 'crypto'
import { join } from 'path'

const [,, trxPath, outDir, layer = 'unit'] = process.argv

// Epic + Feature mapping keyed by test class name suffix
const LABELS = {
  // Auth
  AuthTokenTests:           { epic: 'Auth',     feature: 'Token'          },
  AuthTests:                { epic: 'Auth',     feature: 'Auth API'       },
  // Library
  TitlesTests:              { epic: 'Library',  feature: 'Titles'         },
  ChaptersTests:            { epic: 'Library',  feature: 'Chapters'       },
  ChapterSyncTests:         { epic: 'Library',  feature: 'Chapter Sync'   },
  ProgressTests:            { epic: 'Library',  feature: 'Reading Progress'},
  PatchTitleBodyTests:      { epic: 'Library',  feature: 'Patch API'      },
  MediaLogicTests:          { epic: 'Library',  feature: 'Media'          },
  // Discover
  DiscoverTests:            { epic: 'Discover', feature: 'Search + Add'   },
  DiscoverLogicTests:       { epic: 'Discover', feature: 'Logic'          },
  DiscoverFanOutTests:      { epic: 'Discover', feature: 'Fan-Out'        },
  DiscoverFanOutLogicTests: { epic: 'Discover', feature: 'Fan-Out Logic'  },
  TrendingLaneTests:        { epic: 'Discover', feature: 'Trending Lanes' },
  // Queue
  QueueTests:               { epic: 'Queue',    feature: 'Queue API'      },
  QueueLogicTests:          { epic: 'Queue',    feature: 'Access Control' },
  DownloaderTests:          { epic: 'Queue',    feature: 'Download'       },
  // Settings
  SettingsTests:            { epic: 'Settings', feature: 'Config'         },
  SettingsLogicTests:       { epic: 'Settings', feature: 'Validation'     },
  SourcesTests:             { epic: 'Settings', feature: 'Sources'        },
  PluginsTests:             { epic: 'Settings', feature: 'Plugins'        },
  LogsTests:                { epic: 'Settings', feature: 'Logs'           },
  LogServiceTests:          { epic: 'Settings', feature: 'Logs'           },
  VersionTests:             { epic: 'Settings', feature: 'Version'        },
  UpdateCacheTests:         { epic: 'Settings', feature: 'Update Check'   },
}
if (!trxPath || !outDir) {
  console.error('Usage: trx-to-allure.mjs <results.trx> <allure-results-dir> <layer>')
  process.exit(1)
}

const xml = readFileSync(trxPath, 'utf8')
mkdirSync(outDir, { recursive: true })

function attr(str, name) {
  const m = str.match(new RegExp(`${name}="([^"]*)"`, 'i'))
  return m ? m[1] : ''
}

// Build testId → className map from <TestDefinitions>
const classMap = new Map()
const defMatches = [...xml.matchAll(/<UnitTest\s[^>]*id="([^"]+)"[^>]*>[\s\S]*?<TestMethod[^>]+className="([^"]+)"[^>]*/gi)]
for (const [, id, className] of defMatches) {
  classMap.set(id, className)
}

// Also capture from inline TestMethod blocks
const defBlocks = [...xml.matchAll(/<UnitTest\b[\s\S]*?<\/UnitTest>/gi)]
for (const [block] of defBlocks) {
  const id        = attr(block, 'id')
  const className = attr(block.match(/<TestMethod[^>]*/)?.[0] ?? '', 'className')
  if (id && className) classMap.set(id, className)
}

// Parse <UnitTestResult> entries
const resultMatches = [...xml.matchAll(/<UnitTestResult\s([^>]*)\/>/gi)]

let count = 0

for (const [, attrs] of resultMatches) {
  const testId    = attr(attrs, 'testId')
  const testName  = attr(attrs, 'testName')
  const outcome   = attr(attrs, 'outcome')   // Passed | Failed | Error | NotExecuted
  const duration  = attr(attrs, 'duration')  // HH:MM:SS.fffffff
  const startTime = attr(attrs, 'startTime')
  const endTime   = attr(attrs, 'endTime')

  const className = classMap.get(testId) ?? testName.split('.').slice(0, -1).join('.')
  // className: "ArrghServer.Tests.AuthTokenTests" → suite = "AuthTokenTests"
  const suite = className.split('.').pop() ?? className

  // Parse short method name: full testName may be "ArrghServer.Tests.AuthTokenTests.MyMethod"
  const methodName = testName.split('.').pop() ?? testName

  // Duration "HH:MM:SS.fffffff" → ms
  let durationMs = 0
  const dp = duration.match(/(\d+):(\d+):(\d+)\.?(\d+)?/)
  if (dp) {
    const [, h, m, s, f = '0'] = dp
    durationMs = (parseInt(h) * 3600 + parseInt(m) * 60 + parseInt(s)) * 1000
                 + Math.round(parseInt(f.padEnd(3, '0').slice(0, 3)))
  }

  const start = startTime ? new Date(startTime).getTime() : Date.now() - durationMs
  const stop  = endTime   ? new Date(endTime).getTime()   : start + durationMs

  const status = outcome === 'Passed'      ? 'passed'
               : outcome === 'Failed'      ? 'failed'
               : outcome === 'Error'       ? 'broken'
               : outcome === 'NotExecuted' ? 'skipped'
               : 'unknown'

  const result = {
    uuid:      randomUUID(),
    historyId: `${className}#${methodName}`,
    name:      methodName,
    fullName:  testName,
    status,
    stage: 'finished',
    start,
    stop,
    labels: (() => {
      const map = LABELS[suite] ?? { epic: 'Server', feature: suite }
      return [
        { name: 'layer',       value: layer         },
        { name: 'tag',         value: 'Server'      },
        { name: 'suite',       value: suite         },
        { name: 'parentSuite', value: map.epic      },
        { name: 'epic',        value: map.epic      },
        { name: 'feature',     value: map.feature   },
        { name: 'story',       value: methodName    },
      ]
    })(),
  }

  writeFileSync(join(outDir, `${result.uuid}-result.json`), JSON.stringify(result, null, 2))
  count++
}

console.log(`Converted ${count} test case(s) → ${outDir} [layer=${layer}]`)
