#!/usr/bin/env node
// Converts Hurl JUnit XML output → Allure JSON result files (one per test case).
// Sets layer=api so results appear in the correct tier of the Allure report.
//
// Usage: node junit-to-allure.mjs <junit.xml> <allure-results-dir>

import { readFileSync, writeFileSync, mkdirSync } from 'fs'
import { randomUUID } from 'crypto'
import { join } from 'path'

const [,, junitPath, outDir] = process.argv
if (!junitPath || !outDir) {
  console.error('Usage: junit-to-allure.mjs <junit.xml> <allure-results-dir>')
  process.exit(1)
}

const xml = readFileSync(junitPath, 'utf8')
mkdirSync(outDir, { recursive: true })

// Minimal XML parser — handles Hurl's JUnit output format.
// <testsuite name="..." tests="..." failures="..." time="...">
//   <testcase name="..." classname="..." time="...">
//     <failure message="...">...</failure>   ← optional
//   </testcase>
// </testsuite>

function attr(str, name) {
  const m = str.match(new RegExp(`${name}="([^"]*)"`, 'i'))
  return m ? m[1] : ''
}

const suiteMatches = [...xml.matchAll(/<testsuite\s([^>]*)>([\s\S]*?)<\/testsuite>/gi)]

let count = 0

for (const [, suiteAttrs, suiteBody] of suiteMatches) {
  const suiteName = attr(suiteAttrs, 'name') || 'API'

  const caseMatches = [...suiteBody.matchAll(/<testcase\s([^>]*?)(?:\/>|>([\s\S]*?)<\/testcase>)/gi)]

  for (const [, caseAttrs, caseBody = ''] of caseMatches) {
    const name       = attr(caseAttrs, 'name') || 'unknown'
    const classname  = attr(caseAttrs, 'classname') || suiteName
    const timeStr    = attr(caseAttrs, 'time') || '0'
    const durationMs = Math.round(parseFloat(timeStr) * 1000)

    const failureMatch = caseBody.match(/<failure[^>]*message="([^"]*)"/)
    const errorMatch   = caseBody.match(/<error[^>]*message="([^"]*)"/)

    let status = 'passed'
    let statusDetails = undefined

    if (failureMatch) {
      status = 'failed'
      statusDetails = { message: failureMatch[1], trace: caseBody.replace(/<[^>]+>/g, '').trim() }
    } else if (errorMatch) {
      status = 'broken'
      statusDetails = { message: errorMatch[1], trace: caseBody.replace(/<[^>]+>/g, '').trim() }
    }

    const now  = Date.now()
    const stop = now
    const start = now - durationMs

    // Feature = file stem (e.g. "auth" from "tests/auth.hurl")
    const featureMatch = classname.match(/([^/\\]+?)(?:\.hurl)?$/)
    const feature = featureMatch ? capitalize(featureMatch[1]) : 'API'

    const result = {
      uuid:      randomUUID(),
      historyId: `${classname}#${name}`,
      name,
      status,
      stage:  'finished',
      start,
      stop,
      labels: [
        { name: 'layer',   value: 'api'         },
        { name: 'tag',     value: 'API'          },
        { name: 'suite',   value: suiteName      },
        { name: 'feature', value: feature        },
        { name: 'story',   value: name           },
      ],
      ...(statusDetails && { statusDetails }),
    }

    const filename = join(outDir, `${result.uuid}-result.json`)
    writeFileSync(filename, JSON.stringify(result, null, 2))
    count++
  }
}

console.log(`Converted ${count} test case(s) → ${outDir}`)

function capitalize(s) {
  return s.charAt(0).toUpperCase() + s.slice(1)
}
