import { CalendarCheck2, ChevronLeft, ChevronRight, Menu, Settings2 } from 'lucide-react';
import { useCalendarStore } from '@/stores/calendar-store';
import { Button } from '@/components/ui/button';
import { periodTitle } from '@/features/calendar/presentation';
import type { CalendarView } from '@/types/calendar';

export function Header({ onSettings, firstDay, loading }: { onSettings: () => void; firstDay: 0 | 1; loading: boolean }) {
  const { selectedDate, view, setView, navigate, today, toggleSidebar, sidebarOpen } = useCalendarStore();
  const title = periodTitle(selectedDate, view, firstDay);
  const period = view === 'week' || view === 'week-grid' ? 'semana' : view === 'year' ? 'ano' : 'mês';
  return (
    <header className="app-header">
      <Button variant="ghost" size="icon" aria-label={sidebarOpen ? 'Recolher barra lateral' : 'Expandir barra lateral'} aria-expanded={sidebarOpen} aria-controls="sidebar" onClick={toggleSidebar}><Menu /></Button>
      <div className="brand"><span className="brand-icon"><CalendarCheck2 aria-hidden="true" /></span><span>Manus <strong>Calendar</strong></span></div>
      <Button variant="outline" className="today-button" onClick={today}>Hoje</Button>
      <div className="flex items-center">
        <Button variant="ghost" size="icon" aria-label={`${period} anterior`} onClick={() => navigate(-1)}><ChevronLeft /></Button>
        <Button variant="ghost" size="icon" aria-label={`Próximo período: ${period}`} onClick={() => navigate(1)}><ChevronRight /></Button>
      </div>
      <h1 className="period-title" aria-live="polite" title={title}>{title}</h1>
      <label className="view-select">
        <span className="sr-only">Visualização do calendário</span>
        <select value={view} disabled={loading} aria-label="Visualização do calendário" onChange={(event) => setView(event.target.value as CalendarView)}>
          <option value="week">Semana com horários</option>
          <option value="week-grid">Semana em grade</option>
          <option value="month">Mês</option>
          <option value="year">Ano</option>
        </select>
      </label>
      <Button variant="ghost" size="icon" aria-label="Configurações" onClick={onSettings}><Settings2 /></Button>
    </header>
  );
}
