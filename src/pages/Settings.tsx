import { useEffect, useState } from 'react';
import { api, SchedulerState, SyncAllRow } from '../api';
import Layout from '../components/Layout';
import { Field, LoadingState, useToast } from '../components/ui';

const SETTING_FIELDS: { key: string; label: string; hint: string }[] = [
  { key: 'sync_interval_secs', label: 'Interval sync otomatis (detik, min 60)', hint: 'Dipakai scheduler latar.' },
  { key: 'backup_keep_files', label: 'Jumlah file backup disimpan (1–100)', hint: 'Rotasi otomatis saat backup baru.' },
  { key: 'session_timeout_secs', label: 'Timeout sesi / kunci otomatis (detik, min 60)', hint: 'Berlaku segera untuk sesi baru maupun berjalan.' },
];

const POLICY_FIELDS: { key: string; label: string }[] = [
  { key: 'rounding_minutes', label: 'Pembulatan menit (1 = tanpa pembulatan)' },
  { key: 'max_early_minutes', label: 'Maks. menit datang awal dihitung' },
  { key: 'require_overtime_approval', label: 'Lembur wajib approval (0/1)' },
  { key: 'allow_double_shift', label: 'Izinkan shift ganda (0/1)' },
];

export default function Settings() {
  const toast = useToast();
  const [values, setValues] = useState<Record<string, string>>({});
  const [sched, setSched] = useState<SchedulerState | null>(null);
  const [syncRows, setSyncRows] = useState<SyncAllRow[]>([]);
  const [busy, setBusy] = useState(false);
  const [loading, setLoading] = useState(true);
  const [policy, setPolicy] = useState<Record<string, string>>({});

  const load = async () => {
    try {
      const [s, sc, p] = await Promise.all([
        api.settingsGet(),
        api.schedulerGet(),
        api.tableList('attendance_policy', 1, 1, '', '', 'asc'),
      ]);
      const v: Record<string, string> = {};
      for (const k of Object.keys(s)) v[k] = String(s[k]);
      setValues(v);
      setSched(sc);
      if (p.rows.length > 0) {
        const row = p.rows[0];
        const pv: Record<string, string> = {};
        for (const f of POLICY_FIELDS) pv[f.key] = String(row[f.key] ?? '');
        setPolicy(pv);
      }
    } catch (e) {
      toast.push('error', e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    load().catch(() => {});
  }, []);

  const save = async (key: string) => {
    try {
      await api.settingsSet(key, values[key] ?? '');
      toast.push('success', `Pengaturan ${key} tersimpan`);
      await load();
    } catch (e) {
      toast.push('error', e instanceof Error ? e.message : String(e));
    }
  };

  const toggleScheduler = async (enabled: boolean) => {
    setBusy(true);
    try {
      await api.schedulerSet(enabled);
      toast.push('success', enabled ? 'Scheduler otomatis AKTIF' : 'Scheduler otomatis MATI');
      await load();
    } catch (e) {
      toast.push('error', e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const syncAll = async () => {
    setBusy(true);
    try {
      const rows = await api.syncRunAll();
      setSyncRows(rows);
      const ok = rows.filter((r) => r.ok).length;
      toast.push('success', `Sync semua: ${ok}/${rows.length} perangkat berhasil`);
    } catch (e) {
      toast.push('error', e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const savePolicy = async () => {
    try {
      const cur = await api.tableList('attendance_policy', 1, 1, '', '', 'asc');
      if (cur.rows.length === 0) throw new Error('Baris kebijakan tidak ditemukan');
      await api.tableUpdate('attendance_policy', String(cur.rows[0].id), policy);
      toast.push('success', 'Kebijakan absensi tersimpan (berlaku untuk hitungan berikutnya; hasil lama tidak diubah)');
    } catch (e) {
      toast.push('error', e instanceof Error ? e.message : String(e));
    }
  };

  return (
    <Layout title="Pengaturan" breadcrumb="Sistem">
      {loading ? (
        <LoadingState />
      ) : (
        <div className="row row-deck row-cards">
          <div className="col-lg-6">
            <div className="card">
              <div className="card-header"><h3 className="card-title">Pengaturan sistem (khusus Super Admin)</h3></div>
              <div className="card-body">
                {SETTING_FIELDS.map((f) => (
                  <div className="row g-2 align-items-end mb-3" key={f.key}>
                    <div className="col-8">
                      <Field label={f.label}>
                        <input
                          className="form-control"
                          value={values[f.key] ?? ''}
                          onChange={(e) => setValues((v) => ({ ...v, [f.key]: e.target.value }))}
                        />
                      </Field>
                      <div className="text-secondary small">{f.hint}</div>
                    </div>
                    <div className="col-4">
                      <button className="btn btn-primary w-100" onClick={() => save(f.key)}>Simpan</button>
                    </div>
                  </div>
                ))}
                <div className="text-secondary">
                  Timeout sesi aktif saat ini: {String(values.session_timeout_active ?? '-')} detik.
                </div>
                <hr />
                <h4>Backup otomatis harian</h4>
                <div className="text-secondary mb-2">
                  Saat scheduler aktif dan opsi ini 1, aplikasi membuat backup sendiri
                  sekali sehari (rotasi mengikuti jumlah di atas).
                </div>
                <div className="btn-list">
                  <button className="btn btn-outline-primary" onClick={() => { setValues((v) => ({ ...v, backup_auto: '1' })); }}>Aktifkan (1)</button>
                  <button className="btn btn-outline-secondary" onClick={() => { setValues((v) => ({ ...v, backup_auto: '0' })); }}>Matikan (0)</button>
                  <button className="btn btn-primary" onClick={() => save('backup_auto')}>
                    Simpan (saat ini: {values.backup_auto ?? '?'})
                  </button>
                </div>
              </div>
            </div>
            <div className="card mt-3">
              <div className="card-header"><h3 className="card-title">Kebijakan absensi (berlaku untuk hitungan berikutnya)</h3></div>
              <div className="card-body">
                {POLICY_FIELDS.map((f) => (
                  <Field key={f.key} label={f.label}>
                    <input
                      className="form-control"
                      value={policy[f.key] ?? ''}
                      onChange={(e) => setPolicy((p) => ({ ...p, [f.key]: e.target.value }))}
                    />
                  </Field>
                ))}
                <button className="btn btn-primary" onClick={savePolicy}>Simpan kebijakan</button>
              </div>
            </div>
          </div>
          <div className="col-lg-6">
            <div className="card mb-3">
              <div className="card-header"><h3 className="card-title">Scheduler sinkronisasi otomatis</h3></div>
              <div className="card-body">
                <div className="mb-2">
                  Status: <strong>{sched?.enabled ? 'AKTIF' : 'MATI'}</strong>
                  {' '}· berjalan: {sched?.running ? 'ya' : 'tidak'}
                  {' '}· interval: {sched?.interval_secs} detik
                </div>
                <div className="btn-list">
                  <button className="btn btn-primary" disabled={busy} onClick={() => toggleScheduler(true)}>Aktifkan</button>
                  <button className="btn btn-outline-danger" disabled={busy} onClick={() => toggleScheduler(false)}>Matikan</button>
                </div>
                <div className="text-secondary mt-2">
                  Saat aktif, aplikasi menyinkronkan semua perangkat tiap interval (checkpoint,
                  retry, dan pencatatan sama seperti sync manual) hingga dimatikan.
                </div>
              </div>
            </div>
            <div className="card">
              <div className="card-header"><h3 className="card-title">Sinkronisasi semua perangkat</h3></div>
              <div className="card-body">
                <button className="btn btn-outline-primary" disabled={busy} onClick={syncAll}>
                  {busy ? 'Menyinkronkan…' : 'Sync semua sekarang'}
                </button>
                {syncRows.length > 0 && (
                  <ul className="list-group mt-3">
                    {syncRows.map((r) => (
                      <li className="list-group-item" key={r.device}>
                        <strong>{r.device}</strong> — {r.ok
                          ? `ok (baru ${r.inserted}, duplikat ${r.duplicates}, tanpa mapping ${r.unmapped})`
                          : `GAGAL: ${r.error}`}
                      </li>
                    ))}
                  </ul>
                )}
              </div>
            </div>
          </div>
        </div>
      )}
    </Layout>
  );
}
