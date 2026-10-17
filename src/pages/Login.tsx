import { useEffect, useState } from 'react';
import { useLocation, useNavigate } from 'react-router-dom';
import { api, setSession } from '../api';
import { Field, useToast } from '../components/ui';

export default function Login() {
  const nav = useNavigate();
  const loc = useLocation() as { state?: { from?: string } };
  const toast = useToast();
  const [needsSetup, setNeedsSetup] = useState(false);
  const [username, setUsername] = useState('');
  const [display, setDisplay] = useState('');
  const [password, setPassword] = useState('');
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    api.hasAdmin().then((h) => setNeedsSetup(!h)).catch((e: Error) => toast.push('error', e.message));
  }, []);

  const submit = async (e: React.FormEvent) => {
    e.preventDefault();
    setBusy(true);
    try {
      if (needsSetup) {
        if (password.length < 8) throw new Error('Password minimal 8 karakter');
        await api.setupAdmin(username.trim(), display.trim() || username.trim(), password);
        toast.push('success', 'Admin pertama dibuat. Silakan masuk.');
        setNeedsSetup(false);
        setPassword('');
      } else {
        const res = await api.login(username.trim(), password);
        setSession(res.token, res.username, res.role);
        // Kembali ke halaman yang memaksa redirect (atau dashboard).
        const from = loc.state?.from;
        nav(from && from.startsWith('/') ? from : '/');
      }
    } catch (err) {
      toast.push('error', err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="page page-center">
      <div className="container container-tight py-4">
        <div className="text-center mb-4">
          <span className="brand-badge" style={{ width: 48, height: 48, fontSize: 20 }}>AX</span>
          <h1 className="mt-3">AttendX Rust</h1>
          <p className="text-secondary">
            {needsSetup ? 'Belum ada admin — buat admin pertama (hanya sekali).' : 'Masuk untuk mengelola absensi (WIB, offline).'}
          </p>
        </div>
        <form className="card card-md" onSubmit={submit}>
          <div className="card-body">
            <Field label="Username">
              <input className="form-control" value={username} onChange={(e) => setUsername(e.target.value)} autoComplete="username" required />
            </Field>
            {needsSetup && (
              <Field label="Nama tampilan">
                <input className="form-control" value={display} onChange={(e) => setDisplay(e.target.value)} />
              </Field>
            )}
            <Field label="Password">
              <input
                className="form-control" type="password" value={password}
                onChange={(e) => setPassword(e.target.value)} autoComplete={needsSetup ? 'new-password' : 'current-password'} required
              />
            </Field>
            <button className="btn btn-primary w-100" disabled={busy}>
              {busy ? 'Memproses…' : needsSetup ? 'Buat Admin Pertama' : 'Masuk'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
