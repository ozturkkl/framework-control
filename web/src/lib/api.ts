const API_BASE = (import.meta as any).env?.VITE_API_BASE || 'http://127.0.0.1:8090';

export type PowerResponse = {
  ac_present: boolean;
  battery: null | {
    cycle_count: number;
    charge_percentage: number;
    charging: boolean;
  };
};

export async function getPower(): Promise<PowerResponse> {
  const r = await fetch(`${API_BASE}/api/power`);
  if (!r.ok) throw new Error(`power ${r.status}`);
  return r.json();
}

export async function setFanDuty(percent: number, fan_index: number | null = null): Promise<void> {
  const r = await fetch(`${API_BASE}/api/fan/duty`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ fan_index, percent }),
  });
  if (!r.ok) throw new Error(`fan duty ${r.status}`);
  const j = await r.json();
  if (j.status !== 'ok') throw new Error(j.error || 'failed');
}

export async function checkHealth(): Promise<boolean> {
  try {
    const r = await fetch(`${API_BASE}/api/health`, { method: 'GET' });
    return r.ok;
  } catch (_) {
    return false;
  }
}


