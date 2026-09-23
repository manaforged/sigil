import { execFile } from 'node:child_process';
import { promisify } from 'node:util';

const exec = promisify(execFile);

export async function sample(pid) {
  const timestamp = Date.now();
  if (process.platform === 'win32') {
    const script = `Get-Process -Id ${pid} -ErrorAction Stop | Select-Object CPU,WorkingSet64,PrivateMemorySize64 | ConvertTo-Json -Compress`;
    const { stdout } = await exec('powershell.exe', ['-NoProfile', '-Command', script], { timeout: 3000 });
    const value = JSON.parse(stdout);
    return { timestamp, cpu_seconds: value.CPU, resident_bytes: value.WorkingSet64, private_bytes: value.PrivateMemorySize64 };
  }
  const { stdout } = await exec('ps', ['-p', String(pid), '-o', 'rss=', '-o', 'time='], { timeout: 3000 });
  const match = stdout.trim().match(/^(\d+)\s+(?:(\d+)-)?(?:(\d+):)?(\d+):(\d+(?:\.\d+)?)$/);
  if (!match) throw new Error(`Cannot parse process sample: ${stdout.trim()}`);
  const [, rss, days = 0, hours = 0, minutes, seconds] = match;
  return { timestamp, resident_bytes: Number(rss) * 1024, cpu_seconds: Number(days) * 86400 + Number(hours) * 3600 + Number(minutes) * 60 + Number(seconds) };
}

export async function footprint(pid) {
  if (process.platform !== 'darwin') return null;
  const { stdout } = await exec('vmmap', ['-summary', String(pid)], { timeout: 5000, maxBuffer: 1024 * 1024 });
  const match = stdout.match(/^Physical footprint:\s+([\d.]+)([KMGT]?)$/m);
  if (!match) throw new Error('vmmap did not report a physical footprint');
  const bytes = Number(match[1]) * 1024 ** ('KMGT'.indexOf(match[2]) + 1);
  const categories = {};
  for (const name of ['IOAccelerator (graphics)', 'IOSurface', 'owned unmapped (graphics)']) {
    const line = stdout.split('\n').find(value => value.startsWith(`${name} `));
    const fields = line?.slice(name.length).trim().split(/\s+/);
    if (fields?.[2]) {
      const amount = fields[2].match(/^([\d.]+)([KMGT]?)$/);
      if (amount) categories[name] = Number(amount[1]) * 1024 ** ('KMGT'.indexOf(amount[2]) + 1);
    }
  }
  const zones = stdout.split(/^MALLOC ZONE.*$/m)[1];
  const total = zones?.match(/^TOTAL\s+(.+)$/m)?.[1].trim().split(/\s+/);
  const amount = value => {
    const parsed = value?.match(/^([\d.]+)([KMGT]?)$/);
    return parsed ? Number(parsed[1]) * 1024 ** ('KMGT'.indexOf(parsed[2]) + 1) : null;
  };
  const rows = zones?.split('\n').map(line => line.trim().split(/\s+/))
    .filter(fields => fields.length >= 10 && amount(fields.at(-9)) !== null) ?? [];
  const heap = index => total ? amount(total[index]) : rows.length ? rows.reduce((sum, fields) => sum + amount(fields.at(index - 9)), 0) : null;
  return { timestamp: Date.now(), physical_footprint_bytes: bytes, categories,
    heap_dirty_bytes: heap(2), heap_allocated_bytes: heap(5), raw: stdout };
}

export function memorySummary(memory) {
  if (!memory) return null;
  const samples = memory.samples ?? [];
  const footprints = memory.footprints ?? [];
  const first = samples[0];
  const last = samples.at(-1);
  const elapsed = first && last ? (last.timestamp - first.timestamp) / 1000 : 0;
  return {
    platform: memory.platform,
    machine: memory.machine ?? null,
    vmmap: memory.vmmap ?? footprints.length > 0,
    samples: samples.length,
    resident_peak_bytes: samples.length ? Math.max(...samples.map(value => value.resident_bytes)) : null,
    resident_last_bytes: last?.resident_bytes ?? null,
    physical_footprint_peak_bytes: footprints.length ? Math.max(...footprints.map(value => value.physical_footprint_bytes)) : null,
    physical_footprint_last_bytes: footprints.at(-1)?.physical_footprint_bytes ?? null,
    graphics_categories: footprints.at(-1)?.categories ?? null,
    heap_dirty_bytes: footprints.at(-1)?.heap_dirty_bytes ?? null,
    heap_allocated_bytes: footprints.at(-1)?.heap_allocated_bytes ?? null,
    cpu_percent: elapsed > 0 ? 100 * (last.cpu_seconds - first.cpu_seconds) / elapsed : null,
    errors: memory.errors ?? [],
  };
}
