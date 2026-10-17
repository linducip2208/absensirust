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
  const [serverValues, setServerValues] = useState<Record<string, string>>({});
  const [sched, setSched] = useState<SchedulerState | null>(null);
  const [syncRows, setSyncRows] = useState<SyncAllRow[]>([]);
  const [busy, setBusy] = useState(false);
  const [loading, setLoading] = useState(true);
  const [savingKeys, setSavingKeys] = useState<Record<string, boolean>>({});
  const [policy, setPolicy] = useState<Record<string, string>>({});
  const [policyBusy, setPolicyBusy] = useState(false);

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
      // Bidik nilai server terpisah untuk pemulihan-gagal (tidak menimpa
      // hasil edit yang belum disimpan pada kunci lain).
      setServerValues({ ...v, backup_auto: v.backup_auto ?? '' });
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
    // Muat sekali saat mount; muat ulang dilakukan eksplisit lewat tombol.
    load().catch(() => {});
  }, []);

  const save = async (key: string) => {
    if (savingKeys[key]) return;
    // Angka wajib bulat + rentang wajar (jangan tunggu backend menolak).
    const val = (values[key] ?? '').trim();
    const limits: Record<string, [number, number]> = {
      sync_interval_secs: [60, 86400],
      backup_keep_files: [1, 100],
      session_timeout_secs: [60, 259200],
    };
    const lim = limits[key];
    if (lim && (!/^\d+$/.test(val) || Number(val) < lim[0] || Number(val) > lim[1])) {
      toast.push('error', `${key} harus bilangan bulat ${lim[0]}–${lim[1]}`);
      return;
    }
    setSavingKeys((s) => ({ ...s, [key]: true }));
    try {
      await api.settingsSet(key, val);
      toast.push('success', `Pengaturan ${key} tersimpan`);
      await load();
    } catch (e) {
      // Gagal → kembalikan kotak ke nilai server (bukan menipu "tersimpan").
      setValues((v) => ({ ...v, [key]: serverValues[key] ?? '' }));
      toast.push('error', e instanceof Error ? e.message : String(e));
    } finally {
      setSavingKeys((s) => ({ ...s, [key]: false }));
    }
  };

  const setAndSave = async (key: string, val: string) => {
    if (savingKeys[key]) return;
    setValues((v) => ({ ...v, [key]: val }));
    setSavingKeys((s) => ({ ...s, [key]: true }));
    try {
      await api.settingsSet(key, val);
      toast.push('success', `Pengaturan ${key}=${val} tersimpan`);
      await load();
    } catch (e) {
      setValues((v) => ({ ...v, [key]: serverValues[key] ?? '' }));
      toast.push('error', e instanceof Error ? e.message : String(e));
    } finally {
      setSavingKeys((s) => ({ ...s, [key]: false }));
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
    if (policyBusy) return;
    // Validasi angka sama seperti aturan backend (tapi nangkap duluan).
    for (const f of POLICY_FIELDS) {
      const s = (policy[f.key] ?? '').trim();
      if (f.key === 'require_overtime_approval' || f.key === 'allow_double_shift') {
        if (s !== '0' && s !== '1') {
          toast.push('error', `${f.label}: isi 0 atau 1`);
          return;
        }
        continue;
      }
      if (!/^\d+$/.test(s)) {
        toast.push('error', `${f.label}: harus bilangan bulat`);
        return;
      }
      if (f.key === 'rounding_minutes' && Number(s) < 1) {
        toast.push('error', 'Pembulatan menit minimal 1 (1 = tanpa pembulatan)');
        return;
      }
    }
    setPolicyBusy(true);
    try {
      const cur = await api.tableList('attendance_policy', 1, 1, '', '', 'asc');
      if (cur.rows.length === 0) throw new Error('Baris kebijakan tidak ditemukan');
      await api.tableUpdate('attendance_policy', String(cur.rows[0].id), policy);
      toast.push('success', 'Kebijakan absensi tersimpan (berlaku untuk hitungan berikutnya; hasil lama tidak diubah)');
      await load();
    } catch (e) {
      toast.push('error', e instanceof Error ? e.message : String(e));
    } finally {
      setPolicyBusy(false);
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
                      <button className="btn btn-primary w-100" onClick={() => save(f.key)} disabled={!!savingKeys[f.key]}>
                        {savingKeys[f.key] ? 'Menyimpan…' : 'Simpan'}
                      </button>
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
                  <button className="btn btn-outline-primary" disabled={!!savingKeys.backup_auto} onClick={() => setAndSave('backup_auto', '1')}>
                    Aktifkan (saat ini: {values.backup_auto ?? '?'})
                  </button>
                  <button className="btn btn-outline-secondary" disabled={!!savingKeys.backup_auto} onClick={() => setAndSave('backup_auto', '0')}>Matikan</button>
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
                <button className="btn btn-primary" onClick={savePolicy} disabled={policyBusy}>
                  {policyBusy ? 'Menyimpan…' : 'Simpan kebijakan'}
                </button>
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
