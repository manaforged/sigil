import { spawn } from 'node:child_process';
import { mkdir, open, writeFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { arch, cpus, totalmem } from 'node:os';
import { setTimeout as delay } from 'node:timers/promises';
import { analyze, compare, format } from './profile/report.mjs';
import { footprint, sample } from './profile/process.mjs';

const usage = `node tools/profile.mjs report TRACE_OR_DIRECTORY [--baseline DIRECTORY] [--warmup 10] [--json]
node tools/profile.mjs run --out NEW_DIRECTORY [--timeout 60] [--memory-budget-mib MIB] -- EXECUTABLE ARGS...

run launches an already-built executable with SIGIL_PROFILE enabled and samples its process.
The executable must exit normally and dump its trace before the timeout. No build is run.
Outputs: trace.json, memory.json, process.log, report.json, report.txt, and vmmap.txt on macOS.
Optional: --frame-budget-ms 16.67, --warmup N, --baseline TRACE_OR_DIRECTORY.
Use --no-vmmap for timing comparisons: native memory inspection can pause the target.
Missing native measurements are reported as unavailable, never as zero.`;

function options(args) {
  const values = { warmup: 10, budget: 16.67, memoryBudget: null, timeout: 60, json: false, vmmap: true };
  const names = { '--warmup': 'warmup', '--frame-budget-ms': 'budget', '--memory-budget-mib': 'memoryBudget', '--timeout': 'timeout', '--out': 'out', '--baseline': 'baseline' };
  const positional = [];
  for (let i = 0; i < args.length; i++) {
    const arg = args[i];
    if (arg === '--') return { ...values, positional, command: args.slice(i + 1) };
    if (arg === '--json') { values.json = true; continue; }
    if (arg === '--no-vmmap') { values.vmmap = false; continue; }
    if (names[arg]) {
      const value = args[++i];
      if (value === undefined) throw new Error(`Missing value for ${arg}`);
      values[names[arg]] = ['--out', '--baseline'].includes(arg) ? value : Number(value);
    } else if (arg.startsWith('-')) throw new Error(`Unknown option ${arg}`);
    else positional.push(arg);
  }
  return { ...values, positional, command: [] };
}

function validate(config) {
  for (const name of ['warmup', 'budget', 'timeout', 'memoryBudget']) {
    const value = config[name];
    if (value !== null && (!Number.isFinite(value) || value < 0 || (name !== 'warmup' && value === 0))) throw new Error(`Invalid ${name}`);
  }
  if (!Number.isInteger(config.warmup)) throw new Error('Warmup must be a whole number of frames');
  if (config.timeout > 86400) throw new Error('Timeout must not exceed 86400 seconds');
}

async function capture(config) {
  if (!config.out || !config.command.length) throw new Error('run requires --out and an executable after --');
  const out = resolve(config.out);
  await mkdir(out);
  const log = await open(join(out, 'process.log'), 'wx');
  const memory = { platform: process.platform, machine: { arch: arch(), cpu: cpus()[0]?.model, ram: totalmem() }, vmmap: config.vmmap && process.platform === 'darwin', samples: [], footprints: [], errors: [] };
  const child = spawn(config.command[0], config.command.slice(1), {
    env: { ...process.env, SIGIL_PROFILE: join(out, 'trace.json') }, stdio: ['ignore', log.fd, log.fd],
  });
  memory.pid = child.pid;
  let done = false;
  const completion = new Promise((accept, reject) => {
    child.once('error', reject);
    child.once('close', (code, signal) => accept({ code, signal }));
  }).finally(() => { done = true; });
  let timedOut = false;
  const cancel = () => child.kill('SIGTERM');
  process.once('SIGINT', cancel);
  process.once('SIGTERM', cancel);
  const timer = setTimeout(() => { timedOut = true; child.kill('SIGKILL'); }, config.timeout * 1000);
  const monitor = monitorProcess(child, memory, () => done, out).catch(error => {
    memory.errors.push(`Memory monitor failed: ${error.message}`);
  });
  try {
    const exit = await completion;
    await monitor;
    await writeFile(join(out, 'memory.json'), JSON.stringify(memory, null, 2));
    if (timedOut) throw new Error(`Capture timed out after ${config.timeout}s; incomplete artifacts are in ${out}`);
    if (exit.code !== 0) throw new Error(`Executable failed (${exit.signal ?? exit.code}); see ${join(out, 'process.log')}`);
  } finally {
    clearTimeout(timer);
    process.removeListener('SIGINT', cancel);
    process.removeListener('SIGTERM', cancel);
    await log.close();
  }
  return out;
}

async function monitorProcess(child, memory, done, out) {
  if (!child.pid) return;
  let nextFootprint = Date.now() + 1000;
  while (!done()) {
    try { memory.samples.push(await sample(child.pid)); }
    catch (error) { if (!done()) memory.errors.push(`Process sample unavailable: ${error.message}`); }
    if (memory.vmmap && !done() && Date.now() >= nextFootprint) {
      try {
        const measured = await footprint(child.pid);
        if (measured) {
          const { raw, ...value } = measured;
          memory.footprints.push(value);
          await writeFile(join(out, 'vmmap.txt'), raw);
        }
      } catch (error) { if (!done()) memory.errors.push(`Physical footprint unavailable: ${error.message}`); }
      nextFootprint = Date.now() + 5000;
    }
    if (!done()) await delay(500);
  }
}

async function main() {
  const [mode, ...args] = process.argv.slice(2);
  if (!mode || mode === '--help') { console.log(usage); return; }
  if (!['run', 'report'].includes(mode)) throw new Error(`Unknown command ${mode}`);
  const config = options(args);
  validate(config);
  if (mode === 'report' && config.positional.length !== 1) throw new Error('report requires one trace or capture directory');
  if (mode === 'run' && config.positional.length) throw new Error('Put the executable and its arguments after --');
  const path = mode === 'run' ? await capture(config) : config.positional[0];
  const report = await analyze(path, config);
  if (config.baseline) report.comparison = compare(report, await analyze(config.baseline, config));
  const output = format(report);
  if (mode === 'run') {
    await writeFile(join(path, 'report.json'), JSON.stringify(report, null, 2));
    await writeFile(join(path, 'report.txt'), `${output}\n`);
  }
  console.log(config.json ? JSON.stringify(report, null, 2) : output);
}

main().catch(error => { console.error(`sigil-profile: ${error.message}`); process.exitCode = 1; });
