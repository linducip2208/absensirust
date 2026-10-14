import { useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { Bar, BarChart, CartesianGrid, ResponsiveContainer, Tooltip, XAxis, YAxis } from 'recharts';
import { api, DashboardData, todayWIB } from '../api';
import Layout from '../components/Layout';
import { EmptyState, LoadingState, StatusBadge, useToast } from '../components/ui';

function StatCard({ label, value, color }: { label: string; value: number; color: string }) {
  return (
    <div className="col-sm-6 col-lg-2">
      <div className="card">
        <div className="card-body">
          <div className="subheader">{label}</div>
          <div className={`h1 mb-0 text-${color}`}>{value}</div>
        </div>
      </div>
    </div>
  );
}

export default function Dashboard() {
  const toast = useToast();
  const [data, setData] = useState<DashboardData | null>(null);
  const [date, setDate] = useState(todayWIB());
  const [failed, setFailed] = useState(false);

  const load = async (d: string) => {
    setFailed(false);
    try {
      setData(await api.dashboard(d));
    } catch (e) {
      setFailed(true);
      toast.push('error', e instanceof Error ? e.message : String(e));
    }
  };

  useEffect(() => {
    load(date).catch(() => {});
  }, [date]);

  return (
    <Layout title="Dashboard" breadcrumb="Ringkasan">
      <div className="row mb-3 g-2 align-items-center">
        <div className="col-auto">
          <input type="date" className="form-control" value={date} onChange={(e) => setDate(e.target.value)} />
        </div>
        <div className="col text-secondary">Semua angka dihitung dari database (WIB). Tidak ada angka contoh.</div>
      </div>
      {!data ? (
        failed ? (
          <div className="card"><div className="card-body">
            <EmptyState title="Dashboard gagal dimuat" hint="Periksa koneksi database lalu klik Muat ulang." />
            <button className="btn btn-outline-primary mt-2" onClick={() => load(date)}>Muat ulang</button>
          </div></div>
        ) : (
          <LoadingState text="Memuat dashboard…" />
        )
      ) : (
        <>
          <div className="row row-deck row-cards mb-3">
            <StatCard label="Karyawan aktif" value={data.active_employees} color="primary" />
            <StatCard label="Hadir hari ini" value={data.hadir} color="success" />
            <StatCard label="Terlambat" value={data.terlambat} color="warning" />
            <StatCard label="Tidak hadir" value={data.tidak_hadir} color="danger" />
            <StatCard label="Cuti / izin" value={data.cuti_izin} color="info" />
            <StatCard label="Perangkat online" value={data.devices.online} color="success" />
          </div>
          <div className="row row-deck row-cards">
            <div className="col-lg-7">
              <div className="card">
                <div className="card-header"><h3 className="card-title">Kehadiran 7 hari terakhir</h3></div>
                <div className="card-body" style={{ height: 280 }}>
                  <ResponsiveContainer width="100%" height="100%">
                    <BarChart data={data.daily}>
                      <CartesianGrid strokeDasharray="3 3" />
                      <XAxis dataKey="date" tick={{ fontSize: 11 }} />
                      <YAxis allowDecimals={false} />
                      <Tooltip />
                      <Bar dataKey="hadir" name="Hadir" fill="#2fb344" />
                      <Bar dataKey="terlambat" name="Terlambat" fill="#f76707" />
                    </BarChart>
                  </ResponsiveContainer>
                </div>
              </div>
            </div>
            <div className="col-lg-5">
              <div className="card mb-3">
                <div className="card-header"><h3 className="card-title">Ringkasan shift ({data.date})</h3></div>
                <div className="table-responsive">
                  <table className="table card-table table-vcenter">
                    <thead><tr><th>Kode</th><th>Nama</th><th>Hadir</th></tr></thead>
                    <tbody>
                      {data.shifts.length === 0 && (
                        <tr><td colSpan={3}><EmptyState title="Belum ada shift" hint="Buat shift di menu Shift." /></td></tr>
                      )}
                      {data.shifts.map((s) => (
                        <tr key={s.code}><td>{s.code}</td><td>{s.name}</td><td>{s.count}</td></tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </div>
              <div className="card">
                <div className="card-header"><h3 className="card-title">Perangkat (total {data.devices.total})</h3></div>
                <div className="card-body">
                  <div className="d-flex gap-3 flex-wrap">
                    <span>Online: <strong className="text-success">{data.devices.online}</strong></span>
                    <span>Offline: <strong className="text-secondary">{data.devices.offline}</strong></span>
                    <span>Error: <strong className="text-danger">{data.devices.error}</strong></span>
                  </div>
                  <div className="text-secondary mt-2">Sinkronisasi terakhir: {data.last_sync ?? '-'}</div>
                  {data.alerts.length > 0 && (
                    <div className="alert alert-warning mt-3 mb-0">
                      <strong>Peringatan:</strong> {data.alerts.length} perangkat tidak tersambung:{' '}
                      {data.alerts.map((a) => a.name).join(', ')}
                    </div>
                  )}
                </div>
              </div>
            </div>
          </div>
          <div className="row row-deck row-cards mt-3">
            <div className="col-lg-7">
              <div className="card">
                <div className="card-header"><h3 className="card-title">Aktivitas absensi terbaru</h3></div>
                <div className="table-responsive">
                  <table className="table card-table table-vcenter">
                    <thead><tr><th>Tanggal</th><th>Nama</th><th>Masuk</th><th>Pulang</th><th>Status</th></tr></thead>
                    <tbody>
                      {data.recent.length === 0 && (
                        <tr><td colSpan={5}><EmptyState title="Belum ada aktivitas" hint="Hitung absensi atau sinkronkan perangkat." /></td></tr>
                      )}
                      {data.recent.map((r, i) => (
                        <tr key={i}>
                          <td>{r.date}</td><td>{r.name}</td><td>{r.in ?? '-'}</td><td>{r.out ?? '-'}</td>
                          <td><StatusBadge value={r.status} /></td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </div>
            </div>
            <div className="col-lg-5">
              <div className="card">
                <div className="card-header"><h3 className="card-title">Aksi cepat</h3></div>
                <div className="card-body btn-list">
                  <Link className="btn btn-primary" to="/karyawan">Kelola karyawan</Link>
                  <Link className="btn btn-outline-primary" to="/perangkat">Kelola perangkat</Link>
                  <Link className="btn btn-outline-primary" to="/shift">Kelola shift</Link>
                  <Link className="btn btn-outline-primary" to="/laporan">Buka laporan</Link>
                </div>
              </div>
            </div>
          </div>
        </>
      )}
    </Layout>
  );
}
