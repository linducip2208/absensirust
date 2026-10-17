import Crud, { Column } from '../components/Crud';
import Layout from '../components/Layout';

const USER: Column[] = [
  { key: 'username', label: 'Username', required: true },
  { key: 'display_name', label: 'Nama tampilan', required: true },
  {
    key: 'role', label: 'Role', type: 'select', required: true, badge: true,
    options: [
      { value: 'super_admin', label: 'Super Admin' },
      { value: 'hr_admin', label: 'HR Admin' },
      { value: 'supervisor', label: 'Supervisor' },
      { value: 'operator_device', label: 'Operator Device' },
      { value: 'viewer', label: 'Viewer/Auditor' },
    ],
  },
  {
    key: 'password',
    label: 'Password (wajib saat tambah; saat ubah kosong = tidak diubah)',
    type: 'password', hideInTable: true, requiredOnCreate: true,
  },
  { key: 'is_active', label: 'Aktif', type: 'select', editOnly: true, options: [{ value: '1', label: 'Ya' }, { value: '0', label: 'Tidak' }] },
];

const AUDIT: Column[] = [
  { key: 'created_at', label: 'Waktu (WIB)' },
  { key: 'actor', label: 'Pelaku' },
  { key: 'action', label: 'Aksi' },
  { key: 'entity', label: 'Entitas' },
  { key: 'entity_id', label: 'ID' },
  { key: 'detail', label: 'Detail' },
];

export default function Users() {
  return (
    <Layout title="Pengguna & Audit" breadcrumb="Keamanan">
      <div className="alert alert-info">
        Otorisasi berbasis permission di backend pada setiap operasi. Password di-hash Argon2.
        Tidak ada kredensial default — admin pertama dibuat sekali saat instalasi.
      </div>
      <Crud table="users" title="Pengguna" columns={USER} orderBy="username" />
      <div className="mt-3">
        <Crud table="audit_log" title="Audit log" columns={AUDIT} orderBy="created_at" canWrite={false} />
      </div>
    </Layout>
  );
}
