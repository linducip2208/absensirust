import { useEffect, useState } from 'react';
import { useLocation } from 'react-router-dom';
import { api, downloadFile, getUser, pickFile, Row } from '../api';
import Crud, { Column } from '../components/Crud';
import Layout from '../components/Layout';
import { Field, LoadingState, useToast } from '../components/ui';

const COLUMNS: Column[] = [
  { key: 'nik', label: 'NIK', required: true },
  { key: 'name', label: 'Nama', required: true },
  { key: 'email', label: 'Email' },
  { key: 'phone', label: 'Telepon' },
  { key: 'address', label: 'Alamat', hideInTable: true },
  { key: 'hire_date', label: 'Tgl masuk', required: true },
  { key: 'department_id', label: 'Dept ID' },
  { key: 'position_id', label: 'Jabatan ID' },
  { key: 'branch_id', label: 'Cabang ID' },
  { key: 'manager_id', label: 'Atasan ID', hideInTable: true },
  { key: 'shift_pattern', label: 'Pola shift' },
  { key: 'status', label: 'Status', type: 'select', badge: true, options: [{ value: 'aktif', label: 'Aktif' }, { value: 'nonaktif', label: 'Nonaktif' }] },
];

const MAP_COLUMNS: Column[] = [
  { key: 'employee_id', label: 'Employee ID', required: true },
  { key: 'device_id', label: 'Device ID', required: true },
  { key: 'device_user_id', label: 'User ID di mesin', required: true },
];

export default function Employees() {
  const toast = useToast();
  const loc = useLocation() as { state?: { q?: string } };
  const fromSearch = loc.state?.q ?? '';
  const role = getUser()?.role ?? '';
  const canWrite = ['super_admin', 'hr_admin'].includes(role);
  const [tab, setTab] = useState<'data' | 'map' | 'import'>('data');
  // Kata kunci dari pencarian global: selalu kembali ke tab data karyawan.
  useEffect(() => {
    if (fromSearch) setTab('data');
  }, [fromSearch]);
  const [csv, setCsv] = useState('');
  const [preview, setPreview] = useState<{ headers: string[]; rows: { line: number; data: Record<string, string>; errors: string[] }[]; errors: string[] } | null>(null);
  const [busy, setBusy] = useState(false);
  const [exportBusy, setExportBusy] = useState(false);

  const doPreview = async () => {
    setBusy(true);
    try {
      setPreview(await api.importPreview(csv));
    } catch (e) {
      toast.push('error', e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const doCommit = async () => {
    if (!preview) return;
    const valid = preview.rows.filter((r) => r.errors.length === 0).map((r) => r.data);
    if (valid.length === 0) {
      toast.push('error', 'Tidak ada baris valid untuk diimpor');
      return;
    }
    setBusy(true);
    try {
      const res = await api.importCommit(valid);
      toast.push(res.fail > 0 ? 'info' : 'success', `Impor selesai: ${res.ok} berhasil, ${res.fail} gagal`);
      if (res.messages.length > 0) toast.push('error', res.messages.slice(0, 5).join(' | '));
    } catch (e) {
      toast.push('error', e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const exportCsv = async () => {
    setExportBusy(true);
    try {
      // Halaman demi halaman — dulu hanya halaman-1/200 (sisanya hilang
      // senyap untuk kantor >200 karyawan).
      const head = ['nik', 'name', 'email', 'phone', 'hire_date', 'status'];
      const rows: Row[] = [];
      let page = 1;
      let total = Infinity;
      while (rows.length < total) {
        const res = await api.tableList('employees', page, 200, '', 'name', 'asc');
        total = res.total;
        rows.push(...res.rows);
        if (res.rows.length === 0) break;
        page += 1;
      }
      const lines = [head.join(',')].concat(
        rows.map((r) => head.map((h) => `"${String(r[h] ?? '').replace(/"/g, '""')}"`).join(',')),
      );
      // BOM UTF-8 agar Excel tak salah membaca karakter non-ASCII.
      downloadFile('karyawan.csv', '\uFEFF' + lines.join('\n'), 'text/csv');
      toast.push('success', `Diekspor ${rows.length} karyawan ke CSV`);
    } catch (e) {
      toast.push('error', e instanceof Error ? e.message : String(e));
    } finally {
      setExportBusy(false);
    }
  };

  return (
    <Layout title="Karyawan" breadcrumb="Data master">      <div className="btn-list mb-3">
        <button className={`btn ${tab === 'data' ? 'btn-primary' : ''}`} onClick={() => setTab('data')}>Data karyawan</button>
        <button className={`btn ${tab === 'map' ? 'btn-primary' : ''}`} onClick={() => setTab('map')}>Mapping device</button>
        {canWrite && <button className={`btn ${tab === 'import' ? 'btn-primary' : ''}`} onClick={() => setTab('import')}>Impor CSV</button>}
        <button className="btn btn-outline-primary" disabled={exportBusy} onClick={exportCsv}>
          {exportBusy ? 'Mengekspor…' : 'Ekspor CSV'}
        </button>
      </div>
      {tab === 'data' && <Crud key={`e-${fromSearch}`} table="employees" title="Karyawan" columns={COLUMNS} orderBy="name" canWrite={canWrite} allowHardDelete initialSearch={fromSearch} />}
      {tab === 'data' && canWrite && <PhotoCard />}
      {tab === 'map' && (
        <Crud table="employee_device_map" title="Mapping Karyawan–Device" columns={MAP_COLUMNS} canWrite={canWrite} />
      )}
      {tab === 'import' && canWrite && (
        <div className="card">
          <div className="card-header"><h3 className="card-title">Impor CSV (preview & validasi dulu)</h3></div>
          <div className="card-body">
            <Field label="Tempel isi CSV (header: nik,name,email,phone,hire_date)">
              <textarea className="form-control" rows={8} value={csv} onChange={(e) => setCsv(e.target.value)}
                placeholder={'nik,name,email,phone,hire_date\n001,Budi Santoso,budi@contoh.id,0812…,2024-01-02'} />
            </Field>
            <div className="btn-list">
              <button className="btn btn-outline-primary" disabled={busy || !csv} onClick={doPreview}>
                {busy ? 'Memproses…' : 'Preview & validasi'}
              </button>
              {preview && (
                <button className="btn btn-primary" disabled={busy} onClick={doCommit}>Impor baris valid</button>
              )}
            </div>
            {busy && <LoadingState />}
            {preview && (
              <>
                {preview.errors.length > 0 && (
                  <div className="alert alert-danger mt-3">
                    <strong>{preview.errors.length} masalah:</strong>
                    <ul className="mb-0">{preview.errors.slice(0, 10).map((e, i) => <li key={i}>{e}</li>)}</ul>
                  </div>
                )}
                <div className="table-responsive mt-3">
                  <table className="table table-vcenter">
                    <thead><tr><th>Baris</th><th>NIK</th><th>Nama</th><th>Kesalahan</th></tr></thead>
                    <tbody>
                      {preview.rows.map((r) => (
                        <tr key={r.line} className={r.errors.length > 0 ? 'table-danger' : ''}>
                          <td>{r.line}</td><td>{r.data.nik}</td><td>{r.data.name}</td>
                          <td>{r.errors.join(', ') || 'OK'}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </>
            )}
          </div>
        </div>
      )}
    </Layout>
  );
}

function PhotoCard() {
  const toast = useToast();
  const [emps, setEmps] = useState<Row[]>([]);
  const [sel, setSel] = useState('');
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    api.tableList('employees', 1, 200, '', 'name', 'asc')
      .then((r) => setEmps(r.rows))
      .catch(() => setEmps([]));
  }, []);
  const upload = async () => {
    if (!sel) {
      toast.push('error', 'Pilih karyawan dulu');
      return;
    }
    try {
      const path = await pickFile([{ name: 'Foto', extensions: ['png', 'jpg', 'jpeg'] }]);
      if (!path) return;
      setBusy(true);
      const res = await api.saveEmployeePhoto(sel, path);
      toast.push('success', `Foto tersimpan: ${res.path}`);
    } catch (e) {
      toast.push('error', e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <div className="card mt-3">
      <div className="card-header"><h3 className="card-title">Foto profil (opsional, maks 2 MB)</h3></div>
      <div className="card-body d-flex gap-2 align-items-end flex-wrap">
        <div>
          <label className="form-label">Karyawan</label>
          <select className="form-select" value={sel} onChange={(e) => setSel(e.target.value)}>
            <option value="">— Pilih —</option>
            {emps.map((x) => <option key={String(x.id)} value={String(x.id)}>{String(x.name)}</option>)}
          </select>
        </div>
        <button className="btn btn-outline-primary" disabled={busy} onClick={upload}>
          {busy ? 'Mengunggah…' : 'Pilih & unggah foto'}
        </button>
      </div>
    </div>
  );
}
