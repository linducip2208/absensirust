import { useEffect, useState } from 'react';
import { useLocation } from 'react-router-dom';
import { api, DeviceUser, getToken, RegistryEntry, Row, SdkStatus, SyncStats } from '../api';
import Crud, { Column } from '../components/Crud';
import Layout from '../components/Layout';
import { EmptyState, Field, LoadingState, StatusBadge, useToast } from '../components/ui';

const ADAPTER_OPTIONS = [
  { value: 'zkteco-terminal', label: 'zkteco-terminal (ZKTeco ZKEM)' },
  { value: 'solution-terminal', label: 'solution-terminal (Solution ZKEM)' },
  { value: 'hikvision-terminal', label: 'hikvision-terminal (HCNetSDK)' },
  { value: 'timetech-ac6', label: 'timetech-ac6 (BLOCKED — tanpa SDK)' },
  { value: 'mock-simulated', label: 'mock-simulated (SIMULATED, test saja)' },
];

const COLUMNS: Column[] = [
  { key: 'name', label: 'Nama', required: true },
  { key: 'vendor', label: 'Merek', required: true },
  { key: 'model', label: 'Model' },
  { key: 'serial_number', label: 'Serial', readOnly: true },
  { key: 'host', label: 'IP/Hostname', required: true },
  { key: 'branch_id', label: 'Cabang ID' },
  { key: 'location', label: 'Lokasi' },
  { key: 'connection_type', label: 'Koneksi' },
  { key: 'adapter_name', label: 'Adapter', type: 'select', options: ADAPTER_OPTIONS, required: true },
  {
    key: 'config_json', label: 'Koneksi & kredensial (JSON)', type: 'textarea', hideInTable: true,
  },
  { key: 'sdk_version', label: 'Versi SDK', readOnly: true },
  { key: 'status', label: 'Status', badge: true, readOnly: true },
  { key: 'last_sync_at', label: 'Sync terakhir', readOnly: true },
];

const CONFIG_HINTS: Record<string, string> = {
  'zkteco-terminal': '{"port":4370,"machine":1}',
  'solution-terminal': '{"port":4370,"machine":1}',
  'hikvision-terminal': '{"port":8000,"username":"admin","password":"..."}',
  'timetech-ac6': '{}',
  'mock-simulated': '{}',
};

export default function Devices() {
  const toast = useToast();
  const loc = useLocation() as { state?: { q?: string } };
  const fromSearch = loc.state?.q ?? '';
  const [registry, setRegistry] = useState<RegistryEntry[]>([]);
  const [sdk, setSdk] = useState<SdkStatus[] | null>(null);
  const [testing, setTesting] = useState<string | null>(null);
  const [syncing, setSyncing] = useState<string | null>(null);
  const [lastStats, setLastStats] = useState<SyncStats | null>(null);
  const [refreshKey, setRefreshKey] = useState(0);
  const [logs, setLogs] = useState<Row[]>([]);
  const [registryErr, setRegistryErr] = useState('');
  const [sdkErr, setSdkErr] = useState('');

  useEffect(() => {
    api.deviceRegistry(getToken()).then(setRegistry).catch((e: Error) => {
      // Gagal muat registry: tampilkan pesan jelas, bukan "Memuat" abadi.
      setRegistryErr(e.message);
    });
    api.deviceSdkStatus().then(setSdk).catch((e: Error) => {
      setSdkErr(e.message);
      setSdk([]);
    });
  }, []);

  const test = async (id: string) => {
    setTesting(id);
    try {
      const res = (await api.deviceTest(id)) as { status: string; sdk: string; ms: number };
      toast.push('success', `Test koneksi: ${res.status} (SDK ${res.sdk}, ${res.ms} ms)`);
      setRefreshKey((k) => k + 1);
    } catch (e) {
      toast.push('error', `Test gagal: ${e instanceof Error ? e.message : String(e)}`);
      setRefreshKey((k) => k + 1);
    } finally {
      setTesting(null);
    }
  };

  const sync = async (id: string) => {
    setSyncing(id);
    try {
      const stats = await api.deviceSync(id);
      setLastStats(stats);
      toast.push(
        'success',
        `Sync ${id}: ${stats.inserted} baru, ${stats.duplicates} duplikat, ${stats.unmapped} tanpa mapping` +
          (stats.failed > 0 ? `, ${stats.failed} BARIS KORUP DILEWATI` : ''),
      );
      setRefreshKey((k) => k + 1);
    } catch (e) {
      toast.push('error', `Sync gagal: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      setSyncing(null);
    }
  };

  const showLogs = async () => {
    try {
      const res = await api.tableList('device_connection_log', 1, 30, '', 'created_at', 'desc');
      setLogs(res.rows);
      if (res.rows.length === 0) toast.push('info', 'Belum ada riwayat koneksi.');
    } catch (e) {
      toast.push('error', e instanceof Error ? e.message : String(e));
    }
  };

  const tested = registry.filter((r) => r.status.startsWith('TERSEDIA')).length;

  return (
    <Layout title="Perangkat" breadcrumb="Hardware & SDK">
      <div className="alert alert-info">
        <strong>Status integrasi:</strong> SDK vendor sudah diunduh & adapter nyata
        diimplementasikan ({tested} adapter TERSEDIA: ZKTeco, Solution; Hikvision sebagian;
        TIME TECH AC6 BLOCKED tanpa SDK). Uji hardware fisik masih diperlukan sebelum
        klaim kompatibel — lihat kolom <em>Status uji</em>. Adapter <em>mock-simulated</em> hanya untuk
        pengujian dan selalu ditandai <span className="badge bg-pink-lt simulated-tag">SIMULATED</span>.
      </div>
      <div className="card mb-3">
        <div className="card-header"><h3 className="card-title">Registry adapter & SDK</h3></div>
        <div className="table-responsive">
          <table className="table card-table table-vcenter">
            <thead><tr><th>Adapter</th><th>Vendor</th><th>Model</th><th>SDK</th><th>Operasi</th><th>Status uji</th><th>Kekurangan</th></tr></thead>
            <tbody>
              {registryErr && (
                <tr>
                  <td colSpan={7}>
                    <EmptyState title="Registry gagal dimuat" hint={registryErr} />
                  </td>
                </tr>
              )}
              {!registryErr && registry.length === 0 && (
                <tr><td colSpan={7}><EmptyState title="Memuat registry…" hint="" /></td></tr>
              )}
              {registry.map((r) => (
                <tr key={r.adapter_name}>
                  <td>{r.adapter_name}</td><td>{r.vendor}</td><td>{r.models.join(', ')}</td>
                  <td>{r.sdk_version}</td><td>{r.supported_ops.join(', ') || '—'}</td>
                  <td>{r.status}</td><td>{r.missing}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
      <div className="card mb-3">
        <div className="card-header"><h3 className="card-title">DLL vendor di mesin ini</h3></div>
        <div className="table-responsive">
          <table className="table card-table table-vcenter">
            <thead><tr><th>Adapter</th><th>DLL</th><th>Ditemukan</th><th>Arsitektur</th><th>Siap</th><th>Keterangan</th></tr></thead>
            <tbody>
              {sdk === null && (
                <tr><td colSpan={6}><EmptyState title="Memeriksa DLL…" hint="" /></td></tr>
              )}
              {(sdk ?? []).map((s) => {
                const ready = s.found && s.arch === 'x64' && (s.registered || s.loadable);
                return (
                  <tr key={s.dll}>
                    <td>{s.adapter}</td><td>{s.dll}</td>
                    <td><StatusBadge value={s.found ? 'Ya' : 'Tidak'} /></td>
                    <td>{s.arch || '—'}</td>
                    <td><StatusBadge value={ready ? 'Siap' : 'Belum'} /></td>
                    <td className="text-secondary">{s.found ? s.path : s.hint}</td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
        {sdkErr && <div className="alert alert-danger m-3 mb-0">Status DLL gagal dimuat: {sdkErr}</div>}
        <div className="card-footer text-secondary">
          Taruh DLL di folder exe, <code>exe/sdk</code>, atau <code>%APPDATA%\AttendX\sdk</code>.
          ZKTeco/Solution butuh registrasi sekali (Administrator): <code>regsvr32 zkemkeeper.dll</code>.
          Format <code>config_json</code> per adapter: ZK/Solution <code>{String(CONFIG_HINTS['zkteco-terminal'])}</code>,
          Hikvision <code>{String(CONFIG_HINTS['hikvision-terminal'])}</code> (jangan tulis password di log).
        </div>
      </div>
      <div key={refreshKey}>
        <Crud key={`d-${fromSearch}`} table="devices" title="Perangkat" columns={COLUMNS} orderBy="name" initialSearch={fromSearch} />
      </div>
      <div className="card mt-3">
        <div className="card-header"><h3 className="card-title">Uji & sinkronisasi cepat</h3></div>
        <div className="card-body">
          <DeviceActions key={refreshKey} onTest={test} onSync={sync} testing={testing} syncing={syncing} />
          {lastStats && (
            <div className="alert alert-info mt-3 mb-0">
              Terakhir: {lastStats.device_id} — fetch {lastStats.fetched}, baru {lastStats.inserted},
              duplikat {lastStats.duplicates}, tanpa mapping {lastStats.unmapped}, checkpoint {lastStats.checkpoint || '-'}
            </div>
          )}
          <div className="mt-3">
            <button className="btn btn-outline-primary" onClick={showLogs}>Lihat riwayat koneksi & error</button>
          </div>
          {logs.length > 0 && (
            <div className="table-responsive mt-3">
              <table className="table table-vcenter">
                <thead><tr><th>Waktu (WIB)</th><th>Perangkat</th><th>Event</th><th>Detail</th></tr></thead>
                <tbody>
                  {logs.map((l) => (
                    <tr key={String(l.id)}>
                      <td>{String(l.created_at)}</td><td>{String(l.device_id)}</td>
                      <td><StatusBadge value={l.event} /></td><td>{String(l.detail ?? '').slice(0, 120)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </div>
      </div>
      <DeviceUsers key={refreshKey} />
    </Layout>
  );
}

function DeviceUsers() {
  const toast = useToast();
  const [devices, setDevices] = useState<Row[]>([]);
  const [sel, setSel] = useState('');
  const [users, setUsers] = useState<DeviceUser[]>([]);
  const [uid, setUid] = useState('');
  const [uname, setUname] = useState('');
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    api.tableList('devices', 1, 100, '', 'name', 'asc')
      .then((r) => setDevices(r.rows))
      .catch((e: Error) => toast.push('error', e.message));
  }, [toast]);
  const fetch = async () => {
    if (!sel) {
      toast.push('error', 'Pilih perangkat dulu');
      return;
    }
    setBusy(true);
    try {
      const fetched = await api.deviceFetchUsers(sel);
      setUsers(fetched);
      if (fetched.length === 0) toast.push('info', 'Perangkat tidak melaporkan user (atau capability Unsupported pada SDK nyata)');
    } catch (e) {
      toast.push('error', e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };
  const provision = async () => {
    if (!sel || !uid.trim() || !uname.trim()) {
      toast.push('error', 'Pilih perangkat + isi user ID dan nama');
      return;
    }
    setBusy(true);
    try {
      await api.deviceProvisionUser(sel, uid.trim(), uname.trim());
      toast.push('success', 'User dikirim ke perangkat (bila SDK mendukung provision)');
      setUid('');
      setUname('');
      await fetch();
    } catch (e) {
      toast.push('error', e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <div className="card mt-3">
      <div className="card-header"><h3 className="card-title">User di perangkat — Sync Employees (bila SDK mendukung)</h3></div>
      <div className="card-body">
        <div className="row g-2 align-items-end">
          <div className="col-md-4">
            <Field label="Perangkat">
              <select className="form-select" value={sel} onChange={(e) => setSel(e.target.value)}>
                <option value="">— Pilih —</option>
                {devices.map((d) => <option key={String(d.id)} value={String(d.id)}>{String(d.name)}</option>)}
              </select>
            </Field>
          </div>
          <div className="col-md-8">
            <div className="btn-list">
              <button className="btn btn-outline-primary" disabled={busy} onClick={fetch}>
                {busy ? 'Memuat…' : 'Sync Employees (tarik user)'}
              </button>
            </div>
          </div>
        </div>
        {users.length > 0 && (
          <div className="table-responsive mt-3">
            <table className="table table-vcenter">
              <thead><tr><th>User ID di mesin</th><th>Nama</th></tr></thead>
              <tbody>
                {users.map((u) => (
                  <tr key={u.device_user_id}><td>{u.device_user_id}</td><td>{u.name}</td></tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
        <hr />
        <div className="row g-2 align-items-end">
          <div className="col-md-4"><Field label="User ID baru"><input className="form-control" value={uid} onChange={(e) => setUid(e.target.value)} /></Field></div>
          <div className="col-md-4"><Field label="Nama"><input className="form-control" value={uname} onChange={(e) => setUname(e.target.value)} /></Field></div>
          <div className="col-md-4"><button className="btn btn-outline-primary" disabled={busy} onClick={provision}>Kirim user ke perangkat</button></div>
        </div>
        <div className="text-secondary mt-2">
          Pada adapter nyata yang capability-nya Unsupported, tombol ini mengembalikan error yang jelas
          (bukan berpura-pura berhasil). Cocokkan user mesin ke karyawan via menu Karyawan → Mapping device.
        </div>
      </div>
    </div>
  );
}

function DeviceActions({ onTest, onSync, testing, syncing }: {
  onTest: (id: string) => void; onSync: (id: string) => void;
  testing: string | null; syncing: string | null;
}) {
  const toast = useToast();
  const [devices, setDevices] = useState<Row[]>([]);
  const [loading, setLoading] = useState(true);
  useEffect(() => {
    api.tableList('devices', 1, 100, '', 'name', 'asc')
      .then((r) => setDevices(r.rows))
      .catch((e: Error) => toast.push('error', e.message))
      .finally(() => setLoading(false));
  }, []);
  if (loading) return <LoadingState text="Memuat perangkat…" />;
  if (devices.length === 0) return <div className="text-secondary">Belum ada perangkat. Tambahkan dulu di tabel atas.</div>;
  return (
    <div className="table-responsive">
      <table className="table table-vcenter">
        <thead><tr><th>Perangkat</th><th>Status</th><th>Sync terakhir</th><th>Aksi</th></tr></thead>
        <tbody>
          {devices.map((d) => (
            <tr key={String(d.id)}>
              <td>{String(d.name)} <span className="text-secondary">({String(d.adapter_name)})</span></td>
              <td><StatusBadge value={d.status} /></td>
              <td>{String(d.last_sync_at ?? '-')}</td>
              <td>
                <div className="btn-list flex-nowrap">
                  <button className="btn btn-sm btn-outline-primary" disabled={testing === d.id} onClick={() => onTest(String(d.id))}>
                    {testing === d.id ? 'Menguji…' : 'Test koneksi'}
                  </button>
                  <button className="btn btn-sm btn-primary" disabled={syncing === d.id} onClick={() => onSync(String(d.id))}>
                    {syncing === d.id ? 'Sync…' : 'Sync log'}
                  </button>
                </div>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
