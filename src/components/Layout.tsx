import { NavLink, useNavigate } from 'react-router-dom';
import { useRef, useState } from 'react';
import { api, clearSession, getUser, SearchRow } from '../api';
import { useToast } from './ui';

const MENU: { to: string; label: string; roles: string[] }[] = [
  { to: '/', label: 'Dashboard', roles: ['*'] },
  { to: '/karyawan', label: 'Karyawan', roles: ['super_admin', 'hr_admin', 'supervisor'] },
  { to: '/organisasi', label: 'Cabang & Departemen', roles: ['super_admin', 'hr_admin'] },
  { to: '/shift', label: 'Shift & Jadwal', roles: ['super_admin', 'hr_admin', 'supervisor'] },
  { to: '/jadwal', label: 'Jadwal & Kalender', roles: ['super_admin', 'hr_admin', 'supervisor'] },
  { to: '/absensi', label: 'Absensi', roles: ['*'] },
  { to: '/perangkat', label: 'Perangkat', roles: ['super_admin', 'operator_device'] },
  { to: '/izin', label: 'Izin, Cuti & Lembur', roles: ['super_admin', 'hr_admin', 'supervisor'] },
  { to: '/laporan', label: 'Laporan', roles: ['super_admin', 'hr_admin', 'supervisor', 'viewer'] },
  { to: '/kalender', label: 'Kalender & Libur', roles: ['super_admin', 'hr_admin'] },
  { to: '/backup', label: 'Backup & Kesehatan', roles: ['super_admin'] },
  { to: '/pengaturan', label: 'Pengaturan', roles: ['super_admin'] },
  { to: '/pengguna', label: 'Pengguna', roles: ['super_admin'] },
];

export default function Layout({ children, title, breadcrumb }: { children: React.ReactNode; title: string; breadcrumb?: string }) {
  const nav = useNavigate();
  const toast = useToast();
  const user = getUser();
  const role = user?.role ?? '';
  const roleLabel: Record<string, string> = {
    super_admin: 'Super Admin', hr_admin: 'HR Admin', supervisor: 'Supervisor',
    operator_device: 'Operator Device', viewer: 'Viewer',
  };

  const onLogout = async () => {
    try {
      await api.logout();
    } catch {
      /* abaikan */
    }
    clearSession();
    nav('/login');
  };

  return (
    <div className="page">
      <aside className="navbar navbar-vertical navbar-expand-lg" data-bs-theme="dark">
        <div className="container-fluid">
          <h1 className="navbar-brand navbar-brand-autodark">
            <span className="brand-badge">AX</span>
            <span className="ms-2">AttendX Rust</span>
          </h1>
          <div className="collapse navbar-collapse" id="sidebar-menu">
            <ul className="navbar-nav pt-lg-3">
              <li className="nav-item px-3 pb-2 text-secondary" style={{ fontSize: 12 }}>
                {user?.username} · {roleLabel[role] ?? role}
              </li>
              {MENU.filter((m) => m.roles.includes('*') || m.roles.includes(role)).map((m) => (
                <li className="nav-item" key={m.to}>
                  <NavLink to={m.to} end={m.to === '/'} className={({ isActive }) => `nav-link${isActive ? ' active' : ''}`}>
                    <span className="nav-link-title">{m.label}</span>
                  </NavLink>
                </li>
              ))}
              <li className="nav-item">
                <button
                  className="nav-link"
                  style={{ background: 'none', border: 'none', width: '100%', textAlign: 'left', cursor: 'pointer' }}
                  onClick={() => {
                    onLogout().catch((e: Error) => toast.push('error', e.message));
                  }}
                >
                  <span className="nav-link-title">Keluar</span>
                </button>
              </li>
            </ul>
          </div>
        </div>
      </aside>
      <div className="page-wrapper">
        <div className="page-header d-print-none">
          <div className="container-xl">
            <div className="row g-2 align-items-center">
              <div className="col">
                {breadcrumb && <div className="page-pretitle">{breadcrumb}</div>}
                <h2 className="page-title">{title}</h2>
              </div>
              <div className="col-auto ms-auto">
                <GlobalSearch />
              </div>
            </div>
          </div>
        </div>
        <div className="page-body">
          <div className="container-xl">{children}</div>
        </div>
        <footer className="footer footer-transparent d-print-none">
          <div className="container-xl">
            <div className="row text-center align-items-center flex-row-reverse">
              <div className="col-12 col-lg-auto mt-3 mt-lg-0">
                <ul className="list-inline list-inline-dots mb-0">
                  <li className="list-inline-item text-secondary">AttendX Rust · Zona waktu Asia/Jakarta (WIB) · Offline</li>
                </ul>
              </div>
            </div>
          </div>
        </footer>
      </div>
    </div>
  );
}

function GlobalSearch() {
  const nav = useNavigate();
  const toast = useToast();
  const [q, setQ] = useState('');
  const [rows, setRows] = useState<SearchRow[]>([]);
  const [open, setOpen] = useState(false);
  const [searched, setSearched] = useState(false);
  // Debounce + antrean berurutan: dulu 1 IPC per ketukan; hasil query lama
  // bisa menimpa yang lebih baru (hasil "bud" menggantikan hasil "budi").
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const myReq = useRef(0);
  const go = (v: string) => {
    setQ(v);
    if (timer.current) clearTimeout(timer.current);
    if (v.trim().length < 2) {
      myReq.current += 1; // batalkan in-flight
      setRows([]);
      setOpen(false);
      setSearched(false);
      return;
    }
    timer.current = setTimeout(async () => {
      const id = ++myReq.current;
      try {
        const r = await api.searchGlobal(v.trim());
        if (id !== myReq.current) return;
        setRows([...r.employees, ...r.devices]);
        setSearched(true);
        setOpen(true);
      } catch (e) {
        if (id === myReq.current) {
          toast.push('error', e instanceof Error ? e.message : String(e));
        }
      }
    }, 300);
  };
  return (
    <div style={{ position: 'relative', minWidth: 260 }}>
      <input
        className="form-control"
        placeholder="Cari karyawan / perangkat…"
        value={q}
        onChange={(e) => go(e.target.value)}
        onBlur={() => setTimeout(() => setOpen(false), 200)}
        onFocus={() => rows.length > 0 && setOpen(true)}
      />
      {open && (
        <div className="card" style={{ position: 'absolute', top: '100%', left: 0, right: 0, zIndex: 1000 }}>
          <div className="list-group list-group-flush">
            {rows.map((r) => (
              <button
                key={`${r.kind}-${r.id}`}
                className="list-group-item list-group-item-action"
                onMouseDown={() => {
                  setOpen(false);
                  const to = r.kind === 'karyawan' ? '/karyawan' : '/perangkat';
                  nav(to, { state: { q } });
                }}
              >
                <span className="badge bg-secondary-lt me-2">{r.kind}</span>
                {r.label} <span className="text-secondary">· {r.sub}</span>
              </button>
            ))}
            {searched && rows.length === 0 && (
              <div className="list-group-item text-secondary">Tidak ditemukan untuk “{q}”.</div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
