import { useState } from 'react';
import { api, downloadFile } from '../api';
import Layout from '../components/Layout';
import { Field, LoadingState, useToast } from '../components/ui';

export default function Backup() {
  const toast = useToast();
  const [busy, setBusy] = useState(false);
  const [health, setHealth] = useState<{ app: string; db_version: number; integrity: string; wib_now: string } | null>(null);
  const [restoreFile, setRestoreFile] = useState('');

  const doHealth = async () => {
    try {
      setHealth(await api.healthCheck());
    } catch (e) {
      toast.push('error', e instanceof Error ? e.message : String(e));
    }
  };

  const doBackup = async () => {
    setBusy(true);
    try {
      const res = await api.backupCreate();
      toast.push('success', `Backup tersimpan: ${res.file}`);
    } catch (e) {
      toast.push('error', e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const doRestore = async () => {    if (!restoreFile.trim()) {
      toast.push('error', 'Isi path file backup dulu');
      return;
    }
    if (!confirm('Restore akan MENGGANTI database aktif (salinan pengaman dibuat otomatis). Lanjutkan?')) return;
    setBusy(true);
    try {
      const msg = await api.backupRestore(restoreFile.trim());
      toast.push('success', msg);
    } catch (e) {
      toast.push('error', e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <Layout title="Backup & Kesehatan" breadcrumb="Pemeliharaan">
      <div className="row row-deck row-cards">
        <div className="col-lg-6">
          <div className="card">
            <div className="card-header"><h3 className="card-title">Backup SQLite</h3></div>
            <div className="card-body">
              <div className="btn-list mb-3">
                <button className="btn btn-primary" disabled={busy} onClick={doBackup}>
                  {busy ? 'Memproses…' : 'Buat backup sekarang'}
                </button>
              </div>
              <Field label="Path file backup untuk restore (divalidasi sebelum mengganti DB aktif)">
                <input className="form-control" value={restoreFile} onChange={(e) => setRestoreFile(e.target.value)} placeholder="C:\…\attendx-backup-….db" />
              </Field>
              <button className="btn btn-outline-danger" disabled={busy} onClick={doRestore}>Restore dari file</button>
              {busy && <LoadingState />}
            </div>
          </div>
        </div>
        <div className="col-lg-6">
          <div className="card">
            <div className="card-header"><h3 className="card-title">Kesehatan sistem</h3></div>
            <div className="card-body">
              <button className="btn btn-outline-primary mb-3" onClick={doHealth}>Periksa sekarang</button>
              {health && (
                <dl className="row">
                  <dt className="col-5">Versi aplikasi</dt><dd className="col-7">{health.app}</dd>
                  <dt className="col-5">Versi database</dt><dd className="col-7">{health.db_version}</dd>
                  <dt className="col-5">Integritas</dt><dd className="col-7">{health.integrity}</dd>
                  <dt className="col-5">Waktu WIB</dt><dd className="col-7">{health.wib_now}</dd>
                </dl>
              )}
              <div className="text-secondary">
                Log aplikasi tersimpan di direktori data (rotasi 7 hari, tanpa rahasia).
              </div>
              <button
                className="btn btn-outline-primary mt-2"
                onClick={async () => {
                  try {
                    const b = await api.diagnosticsBundle();
                    downloadFile('diagnostik-attendx.txt', b.text, 'text/plain');
                    toast.push('success', 'Bundel diagnostik diunduh (tanpa password/token/biometrik)');
                  } catch (e) {
                    toast.push('error', e instanceof Error ? e.message : String(e));
                  }
                }}
              >
                Unduh bundel diagnostik
              </button>
            </div>
          </div>
        </div>
      </div>
    </Layout>
  );
}
