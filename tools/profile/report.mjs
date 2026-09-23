import { readFile, stat } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { createHash } from 'node:crypto';
import { memorySummary } from './process.mjs';

export function distribution(values) {
  const sorted = values.toSorted((a, b) => a - b);
  if (!sorted.length) return null;
  const percentile = p => sorted[Math.max(0, Math.ceil(sorted.length * p) - 1)];
  return { n: sorted.length, p50: percentile(0.5), p95: percentile(0.95), p99: percentile(0.99), max: sorted.at(-1), mean: sorted.reduce((a, b) => a + b, 0) / sorted.length };
}

async function optional(path, json = true) {
  try {
    const value = await readFile(path);
    return json ? JSON.parse(value) : value;
  } catch (error) {
    if (error.code === 'ENOENT') return null;
    throw error;
  }
}

export async function analyze(path, options) {
  if ((await stat(path)).isDirectory()) path = join(path, 'trace.json');
  const trace = JSON.parse(await readFile(path, 'utf8'));
  if (!Array.isArray(trace.traceEvents)) throw new Error(`${path}: not a Sigil trace`);
  const metadata = trace.metadata ?? {};
  if (metadata.schema && metadata.schema !== 1) throw new Error(`Unsupported trace schema ${metadata.schema}`);
  const memory = await optional(join(dirname(path), 'memory.json'));
  if (memory?.pid && metadata.pid && memory.pid !== metadata.pid) throw new Error('Memory samples and trace belong to different processes');
  const rawFrames = trace.frames ?? legacyFrames(trace.traceEvents);
  const frames = rawFrames?.slice(options.warmup);
  if (frames && !frames.length) throw new Error('No frames remain after warmup; reduce --warmup');
  const phases = new Map();
  const counters = new Map();
  const add = (map, name, value) => {
    if (!Number.isFinite(value) || value < 0) throw new Error(`Invalid metric ${name}`);
    if (!map.has(name)) map.set(name, []);
    map.get(name).push(value);
  };
  if (frames) {
    const names = new Set(rawFrames.flatMap(frame => frame.counts.map(([name]) => name)));
    for (const frame of frames) {
      add(phases, 'frame', frame.wall_ns / 1e6);
      for (const [name, ns] of frame.phases_ns) add(phases, name, ns / 1e6);
      const counts = Object.fromEntries(frame.counts);
      for (const name of names) {
        if (name.startsWith('wgpu.ids.') && counts[name] === undefined) continue;
        add(counters, name, counts[name] ?? 0);
      }
    }
  } else {
    for (const event of trace.traceEvents) {
      if (event.ph === 'X') add(phases, event.name === 'gpu' ? 'render.cpu' : event.name, event.dur / 1000);
      if (event.ph === 'C') add(counters, event.name, event.args.value);
    }
  }
  const result = {
    source: path, metadata, warmup_frames: frames ? options.warmup : null,
    frames: frames?.length ?? null,
    phases_ms: Object.fromEntries([...phases].map(([name, values]) => [name, distribution(values)])),
    counters: Object.fromEntries([...counters].map(([name, values]) => [name, distribution(values)])),
    causes: {},
    frame_budget_ms: options.budget,
    over_budget_frames: frames ? frames.filter(frame => frame.wall_ns / 1e6 > options.budget).length : null,
    memory: memorySummary(memory),
    findings: [], limitations: [],
  };
  for (const frame of frames ?? []) result.causes[frame.cause] = (result.causes[frame.cause] ?? 0) + 1;
  result.host = counters.has('tui.frames') ? 'tui' : 'gui';
  const capture = await optional(path.replace(/\.json$/, result.host === 'tui' ? '.ansi' : '.png'), false);
  result.capture_sha256 = capture ? createHash('sha256').update(capture).digest('hex') : null;
  diagnose(result, options);
  return result;
}

function legacyFrames(events) {
  const frames = events.filter(event => event.ph === 'X' && event.name === 'frame').sort((a, b) => a.ts - b.ts);
  if (!frames.length) return undefined;
  return frames.map((frame, index) => {
    const next = frames[index + 1]?.ts ?? Infinity;
    const within = events.filter(event => event.ts >= frame.ts && event.ts < next);
    return {
      index, wall_ns: frame.dur * 1000,
      cause: within.find(event => event.ph === 's')?.name ?? 'unknown',
      phases_ns: within.filter(event => event.ph === 'X' && event.name !== 'frame').map(event => [event.name, event.dur * 1000]),
      counts: within.filter(event => event.ph === 'C').map(event => [event.name, event.args.value]),
    };
  });
}

function diagnose(report, options) {
  const { phases_ms: phases, counters, findings, limitations, metadata } = report;
  if (!report.frames) limitations.push('Legacy trace: frame boundaries, warmup exclusion, and loss counts are unavailable.');
  else if (!metadata.schema) limitations.push('Legacy trace: frames reconstructed from explicit frame spans; capture loss metadata is unavailable.');
  if (metadata.dropped_frames || metadata.dropped_events) limitations.push(`Capture truncated: ${metadata.dropped_frames} frames and ${metadata.dropped_events} events dropped.`);
  if (metadata.debug_assertions) limitations.push('Debug build: do not treat these timings as release performance.');
  if (report.memory) limitations.push('Memory samples cover the whole command, including startup and image capture; warmup exclusion applies to frame metrics only.');
  limitations.push('Timings are CPU wall time. Nested phases must not be summed.');
  if (report.host === 'gui') limitations.push('GPU execution time is not measured. Allocation counters cover instrumented draw/upload buffers and layer textures, not every driver resource. Bytes are requested payloads, not driver overhead.');
  if (report.host === 'gui') limitations.push('wgpu.ids counters are registry IDs, not a complete census of driver-retained allocations.');
  if (report.host === 'tui') limitations.push('Terminal write timing measures the supplied writer; headless captures exclude terminal emulator rendering. Output equality checks the final ANSI capture.');
  if (counters['workload.headless_serialized']) limitations.push('Headless render workload waits after each submission. It does not measure window presentation, idle behavior, or input latency.');
  if (counters['cpu.thread_allocations']) limitations.push('Rust allocation counts cover the calling thread, including instrumentation; native driver allocations are excluded.');
  if (!report.capture_sha256) limitations.push('No image capture: rendering equivalence has not been checked.');
  const work = ['effect', 'layout', 'emit', 'render.cpu', 'render.wait', 'a11y', 'app.update', 'tui.frame', 'tui.paint', 'tui.raster', 'tui.write']
    .filter(name => phases[name]).sort((a, b) => phases[b].p95 - phases[a].p95);
  for (const name of work.slice(0, 3)) {
    const value = phases[name];
    findings.push({ kind: 'cpu', metric: name, evidence: `p95 ${value.p95.toFixed(3)} ms; max ${value.max.toFixed(3)} ms`, action: name === 'render.cpu' ? 'Inspect render passes and resource allocations below; use a GPU capture to separate driver cost from GPU execution.' : `Inspect ${name} work in the retained frames.`, budget_exceeded: value.p95 > options.budget });
  }
  for (const name of ['alloc.sprite.buffers', 'alloc.raster.buffers', 'alloc.upload.buffers', 'alloc.layer.textures']) {
    const value = counters[name];
    if (value?.mean > 0) findings.push({ kind: 'allocation', metric: name, evidence: `${value.mean.toFixed(1)} per ${report.frames ? 'sampled frame after warmup' : 'recorded nonzero sample'}; max ${value.max}`, action: name === 'alloc.layer.textures' ? 'Check layer dimensions and pool reuse; allocations still occur in the measured window.' : 'Reuse buffers across draws and frames in the existing upload path.' });
  }
  if (counters['gpu.layers']?.mean > 0) findings.push({ kind: 'compositing', metric: 'gpu.layers', evidence: `${counters['gpu.layers'].mean.toFixed(1)} offscreen layers per ${report.frames ? 'sampled frame' : 'recorded nonzero sample'}`, action: 'Inspect rounded clipping and effects. Layer counts establish work, not whether each layer is avoidable.' });
  if (counters['tui.bytes']) findings.push({ kind: 'terminal', metric: 'tui.bytes', evidence: `${counters['tui.bytes'].mean.toFixed(1)} output bytes per measured frame; max ${counters['tui.bytes'].max}`, action: 'Check idle frames for writes and compare streaming output with changed rows; use the ANSI capture to check output.' });
  const memory = report.memory;
  if (memory) {
    const bytes = memory.physical_footprint_peak_bytes ?? memory.resident_peak_bytes;
    if (bytes !== null && options.memoryBudget !== null && bytes > options.memoryBudget * 1024 ** 2) findings.unshift({ kind: 'memory', metric: memory.physical_footprint_peak_bytes !== null ? 'physical_footprint_peak' : 'resident_peak', evidence: `${(bytes / 1024 ** 2).toFixed(1)} MiB exceeds configured ${options.memoryBudget} MiB budget`, action: 'Separate display surfaces, other graphics allocations, and heap using the captured native memory report.', budget_exceeded: true });
    const categories = Object.entries(memory.graphics_categories ?? {}).sort((a, b) => b[1] - a[1]);
    if (categories[0]?.[1] > 0) findings.push({ kind: 'memory', metric: categories[0][0], evidence: `${(categories[0][1] / 1024 ** 2).toFixed(1)} MiB dirty in the last native snapshot`, action: 'Inspect render targets, image/glyph atlases, and driver-retained resources; registry IDs alone cannot account for these bytes.' });
    if (memory.heap_allocated_bytes !== null) findings.push({ kind: 'memory', metric: 'heap', evidence: `${(memory.heap_allocated_bytes / 1024 ** 2).toFixed(1)} MiB allocated; ${(memory.heap_dirty_bytes / 1024 ** 2).toFixed(1)} MiB dirty`, action: 'Use allocation stacks to attribute live heap objects; frame allocation counts measure churn, not retained bytes.' });
    if (memory.vmmap) limitations.push('vmmap inspection can pause the target and distort frame timing. Recapture with --no-vmmap before comparing timings.');
    limitations.push('Process samples include instrumentation overhead and exclude child processes. RSS/working set and physical footprint are different measurements.');
    limitations.push(...memory.errors);
  } else limitations.push('No process samples: total RAM and CPU utilization are unknown.');
}

export function compare(current, baseline) {
  const reasons = [];
  for (const name of ['os', 'arch', 'debug_assertions']) {
    if (current.metadata[name] === undefined || current.metadata[name] !== baseline.metadata[name]) reasons.push(`Missing or different ${name}`);
  }
  if (current.host !== baseline.host) reasons.push('Different rendering hosts');
  const workload = current.host === 'tui' ? ['workload.cols', 'workload.rows', 'workload.items', 'workload.scenario'] : ['workload.width', 'workload.height', 'workload.scale', 'workload.rows'];
  for (const name of workload) {
    const a = current.counters[name];
    const b = baseline.counters[name];
    if (!a || !b || a.p50 !== b.p50 || a.max !== b.max) reasons.push(`Missing or different ${name}`);
  }
  if (current.warmup_frames !== baseline.warmup_frames || current.frames !== baseline.frames) reasons.push('Different measured frame windows');
  if (!current.capture_sha256 || current.capture_sha256 !== baseline.capture_sha256) reasons.push('Captured output equality is unverified or differs');
  if (current.metadata.dropped_frames || baseline.metadata.dropped_frames) reasons.push('Frame capture was truncated');
  if (current.memory?.vmmap || baseline.memory?.vmmap) reasons.push('Native memory inspection can pause the target; recapture both runs with --no-vmmap');
  const machine = current.memory?.machine;
  if (!machine || !baseline.memory?.machine || ['arch', 'cpu', 'ram'].some(name => machine[name] !== baseline.memory.machine[name])) reasons.push('Missing or different machine configuration');
  const deltas = {};
  for (const [name, value] of Object.entries(current.phases_ms)) {
    const before = baseline.phases_ms[name]?.p95;
    if (before !== undefined) deltas[name] = { baseline_p95_ms: before, current_p95_ms: value.p95, delta_ms: value.p95 - before };
  }
  return { comparable: reasons.length === 0, reasons, phases: deltas };
}

export function format(report) {
  const lines = [
    `SIGIL PERFORMANCE REPORT (${report.host})`,
    report.source,
    `${report.frames ?? 'unknown'} measured frames; ${report.warmup_frames ?? 'unknown'} warmup frames excluded; budget ${report.frame_budget_ms} ms`,
    `Over budget: ${report.over_budget_frames ?? 'unknown'}`,
    `Frame causes: ${Object.entries(report.causes).map(([name, count]) => `${name}=${count}`).join(', ') || 'unavailable'}`,
    '', 'CPU wall time (ms)                 p50       p95       p99       max',
  ];
  for (const [name, value] of Object.entries(report.phases_ms)) {
    lines.push(`${name.padEnd(29)}${[value.p50, value.p95, value.p99, value.max].map(n => n.toFixed(3).padStart(10)).join('')}`);
  }
  lines.push('', report.frames ? 'COUNTERS: mean per measured frame / maximum; registry IDs per snapshot' : 'COUNTERS: mean per recorded sample / maximum');
  for (const [name, value] of Object.entries(report.counters)) lines.push(`${name}: ${value.mean.toFixed(1)} / ${value.max}`);
  if (report.memory) {
    lines.push('', 'PROCESS MEMORY (peaks are sampled, not lifetime maxima)');
    for (const name of ['resident_peak_bytes', 'physical_footprint_peak_bytes', 'physical_footprint_last_bytes', 'heap_allocated_bytes', 'heap_dirty_bytes']) {
      const value = report.memory[name];
      lines.push(`${name}: ${value === null ? 'unavailable' : `${(value / 1024 ** 2).toFixed(1)} MiB`}`);
    }
    lines.push(`CPU utilization: ${report.memory.cpu_percent?.toFixed(1) ?? 'unavailable'}% of one core`);
    for (const [name, value] of Object.entries(report.memory.graphics_categories ?? {})) lines.push(`${name}: ${(value / 1024 ** 2).toFixed(1)} MiB`);
  }
  lines.push('', 'FINDINGS');
  report.findings.forEach((finding, index) => lines.push(`${index + 1}. ${finding.metric}: ${finding.evidence}\n   ${finding.action}`));
  if (report.comparison) {
    lines.push('', `BASELINE: ${report.comparison.comparable ? 'same workload and captured output' : 'not comparable'}`);
    lines.push(...report.comparison.reasons);
    if (report.comparison.comparable) for (const [name, value] of Object.entries(report.comparison.phases)) lines.push(`${name} p95: ${value.baseline_p95_ms.toFixed(3)} -> ${value.current_p95_ms.toFixed(3)} ms`);
  }
  lines.push('', 'LIMITATIONS', ...report.limitations.map(value => `- ${value}`));
  return lines.join('\n');
}
