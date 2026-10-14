import { useState } from 'react';
import { api, downloadFile, pickFile } from '../api';
import Layout from '../components/Layout';
import { ConfirmDialog, Field, LoadingState, useToast } from '../components/ui';

export default function Backup() {
  const toast = useToast();
  const [busy, setBusy] = useState(false);
  const [health, setHealth] = useState<{ app: string; db_version: number; integrity: string; wib_now: string } | null>(null);
  const [restoreFile, setRestoreFile] = useState('');
  const [confirmRestore, setConfirmRestore] = useState(false);

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

  const pickRestore = async () => {
    try {
      const p = await pickFile([{ name: 'Backup SQLite', extensions: ['db'] }]);
      if (p) setRestoreFile(p);
    } catch (e) {
      toast.push('error', e instanceof Error ? e.message : String(e));
    }
  };

  const doRestore = async () => {    if (!restoreFile.trim()) {
      toast.push('error', 'Pilih file backup dulu');
      return;
    }
    setConfirmRestore(true);
  };

  const confirmDoRestore = async () => {
    setConfirmRestore(false);
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
              <Field label="File backup untuk restore (hanya dari folder backups, divalidasi dulu)">
                <div className="d-flex gap-2">
                  <input className="form-control" value={restoreFile} onChange={(e) => setRestoreFile(e.target.value)} placeholder="Pilih via tombol…" />
                  <button className="btn btn-outline-primary text-nowrap" onClick={pickRestore}>Pilih file…</button>
                </div>
              </Field>
              <button className="btn btn-outline-danger" disabled={busy} onClick={doRestore}>Restore dari file</button>
              {busy && <LoadingState />}
              {confirmRestore && (
                <ConfirmDialog
                  title="Restore database?"
                  body="Restore akan MENGGANTI database aktif (salinan pengaman dibuat otomatis, file divalidasi dulu)."
                  confirmLabel="Ya, restore"
                  onCancel={() => setConfirmRestore(false)}
                  onConfirm={confirmDoRestore}
                />
              )}
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
