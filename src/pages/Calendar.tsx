import { useState } from 'react';
import Crud, { Column } from '../components/Crud';
import Layout from '../components/Layout';

const HOLIDAY: Column[] = [
  { key: 'holiday_date', label: 'Tanggal', type: 'date', required: true },
  { key: 'name', label: 'Nama libur', required: true },
  { key: 'kind', label: 'Jenis', type: 'select', options: [{ value: 'nasional', label: 'Nasional' }, { value: 'perusahaan', label: 'Perusahaan' }] },
];
const CAL: Column[] = [
  { key: 'work_date', label: 'Tanggal', type: 'date', required: true },
  { key: 'kind', label: 'Jenis', type: 'select', required: true, options: [{ value: 'kerja', label: 'Kerja' }, { value: 'libur', label: 'Libur' }] },
  { key: 'description', label: 'Keterangan' },
];

export default function Calendar() {
  const [tab, setTab] = useState<'libur' | 'kalender'>('libur');
  return (
    <Layout title="Kalender & Libur" breadcrumb="Waktu kerja">
      <div className="btn-list mb-3">
        <button className={`btn ${tab === 'libur' ? 'btn-primary' : ''}`} onClick={() => setTab('libur')}>Hari libur</button>
        <button className={`btn ${tab === 'kalender' ? 'btn-primary' : ''}`} onClick={() => setTab('kalender')}>Kalender kerja</button>
      </div>
      {tab === 'libur' && <Crud table="holidays" title="Hari libur" columns={HOLIDAY} orderBy="holiday_date" />}
      {tab === 'kalender' && <Crud table="work_calendar" title="Kalender kerja" columns={CAL} orderBy="work_date" />}
    </Layout>
  );
}
