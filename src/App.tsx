import { Navigate, Route, Routes, useLocation } from 'react-router-dom';
import Layout from './components/Layout';
import { ToastProvider } from './components/ui';
import { getToken } from './api';
import Attendance from './pages/Attendance';
import Backup from './pages/Backup';
import Calendar from './pages/Calendar';
import Dashboard from './pages/Dashboard';
import Devices from './pages/Devices';
import Employees from './pages/Employees';
import Leaves from './pages/Leaves';
import Login from './pages/Login';
import Organization from './pages/Organization';
import Reports from './pages/Reports';
import Schedule from './pages/Schedule';
import Settings from './pages/Settings';
import Shifts from './pages/Shifts';
import Users from './pages/Users';

function Guard({ children }: { children: JSX.Element }) {
  const loc = useLocation();
  if (!getToken()) return <Navigate to="/login" state={{ from: loc.pathname }} replace />;
  return children;
}

function NotFound() {
  return (
    <Layout title="Tidak ditemukan" breadcrumb="Error">
      <div className="card"><div className="card-body">Halaman tidak tersedia.</div></div>
    </Layout>
  );
}

export default function App() {
  return (
    <ToastProvider>
      <Routes>
        <Route path="/login" element={<Login />} />
        <Route path="/" element={<Guard><Dashboard /></Guard>} />
        <Route path="/karyawan" element={<Guard><Employees /></Guard>} />
        <Route path="/organisasi" element={<Guard><Organization /></Guard>} />
        <Route path="/shift" element={<Guard><Shifts /></Guard>} />
        <Route path="/jadwal" element={<Guard><Schedule /></Guard>} />
        <Route path="/absensi" element={<Guard><Attendance /></Guard>} />
        <Route path="/perangkat" element={<Guard><Devices /></Guard>} />
        <Route path="/izin" element={<Guard><Leaves /></Guard>} />
        <Route path="/laporan" element={<Guard><Reports /></Guard>} />
        <Route path="/kalender" element={<Guard><Calendar /></Guard>} />
        <Route path="/backup" element={<Guard><Backup /></Guard>} />
        <Route path="/pengaturan" element={<Guard><Settings /></Guard>} />
        <Route path="/pengguna" element={<Guard><Users /></Guard>} />
        <Route path="*" element={<Guard><NotFound /></Guard>} />
      </Routes>
    </ToastProvider>
  );
}
