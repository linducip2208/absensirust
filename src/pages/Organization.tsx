import { useState } from 'react';
import Crud, { Column } from '../components/Crud';
import Layout from '../components/Layout';

const BRANCH: Column[] = [
  { key: 'code', label: 'Kode', required: true },
  { key: 'name', label: 'Nama cabang', required: true },
  { key: 'address', label: 'Alamat' },
];
const DEPT: Column[] = [
  { key: 'code', label: 'Kode', required: true },
  { key: 'name', label: 'Nama departemen', required: true },
  { key: 'branch_id', label: 'Cabang ID' },
];
const POS: Column[] = [
  { key: 'code', label: 'Kode', required: true },
  { key: 'name', label: 'Nama jabatan', required: true },
];

export default function Organization() {
  const [tab, setTab] = useState<'cabang' | 'dept' | 'jabatan'>('cabang');
  return (
    <Layout title="Cabang & Departemen" breadcrumb="Data master">
      <div className="btn-list mb-3">
        <button className={`btn ${tab === 'cabang' ? 'btn-primary' : ''}`} onClick={() => setTab('cabang')}>Cabang</button>
        <button className={`btn ${tab === 'dept' ? 'btn-primary' : ''}`} onClick={() => setTab('dept')}>Departemen</button>
        <button className={`btn ${tab === 'jabatan' ? 'btn-primary' : ''}`} onClick={() => setTab('jabatan')}>Jabatan</button>
      </div>
      {tab === 'cabang' && <Crud table="branches" title="Cabang" columns={BRANCH} orderBy="name" />}
      {tab === 'dept' && <Crud table="departments" title="Departemen" columns={DEPT} orderBy="name" />}
      {tab === 'jabatan' && <Crud table="positions" title="Jabatan" columns={POS} orderBy="name" />}
    </Layout>
  );
}
